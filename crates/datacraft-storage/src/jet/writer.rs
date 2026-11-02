//! Jet4 writer core (step 10.1, item H9): read-modify-write over the raw
//! file bytes with usage-map-consistent page allocation.
//!
//! Allocation is **append-only**: a new page is taken at EOF, its bit is
//! cleared in the global free map (page 1, row 0) and set in the owning
//! table's used map. Interior free-page reclaim is a documented later
//! optimization — the fixtures carry no interior free pages. Map rows are
//! rewritten in place; growth keeps every `(page, row)` reference valid
//! because rows keep their slot indices (`docs/FORMAT-JET.md`
//! "Usage maps (writer)").

use crate::error::StorageError;
use crate::jet::data_page::DataPage;
use crate::jet::header::FileHeader;
use crate::jet::page::{PAGE_TYPE_DATA, PAGE_TYPE_TDEF, page_type};
use crate::jet::usage_map::{MapRef, UsageMap};

/// Phony TDEF page of the usage-map table itself (page 1 owns its own row).
pub const USAGE_TABLE_ROOT: u32 = 1;
/// TDEF block offset of the `used_pages` dword (`docs/FORMAT-JET.md`,
/// "Table definition pages").
const TDEF_USED_PAGES_AT: usize = 47;

/// Read-modify-write access to a Jet4/ACE file buffer.
#[derive(Debug)]
pub struct Jet4Writer {
    pub(crate) header: FileHeader,
    pub(crate) file: Vec<u8>,
    /// Timestamp for new catalog rows (`DateCreate`/`DateUpdate`). Defaults
    /// to the construction time; the deterministic template materializer
    /// pins it (phase 29, step 29.2 — LESSONS #6).
    pub(crate) catalog_date: f64,
}

impl Jet4Writer {
    /// Takes ownership of the raw file bytes and validates the header.
    /// Jet3 is rejected (BACKLOG L1), like the reader, and office-encrypted
    /// ACE files are rejected precisely (step 16.2, M5) — the writer must
    /// never operate on encrypted pages.
    pub fn new(file: Vec<u8>) -> Result<Self, StorageError> {
        let header = FileHeader::decode(&file, file.len() as u64)?;
        if header.format == crate::types::FormatVersion::Jet3 {
            return Err(StorageError::Unsupported {
                reason: "Jet3 (Access 97) databases are not supported yet (BACKLOG L1)".into(),
            });
        }
        // The descriptor length at 0x299 is outside the RC4 header region,
        // so this check holds for raw on-disk bytes and for the decrypted
        // writer handoff alike; db-key/password forms cannot reach here —
        // the reader boundary validates them at open.
        if header.format == crate::types::FormatVersion::Ace
            && crate::jet::crypto::office_descriptor_len(&file) != 0
        {
            return Err(StorageError::Unsupported {
                reason: "ACE Office encryption (RC4 CryptoAPI/AES) is not supported — \
                         DataCraft reads Jet3/Jet4 RC4 databases only"
                    .to_owned(),
            });
        }
        Ok(Self {
            header,
            file,
            catalog_date: crate::jet::encode::now_as_ole_date(),
        })
    }

    /// Pins the timestamp new catalog rows carry (deterministic
    /// materialization, phase 29 step 29.2 — LESSONS #6).
    pub fn set_catalog_date(&mut self, date: f64) {
        self.catalog_date = date;
    }

    /// The decoded file header (`page_count` tracks file growth).
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Borrows the raw file bytes.
    pub fn file(&self) -> &[u8] {
        &self.file
    }

    /// Consumes the writer and returns the written file bytes.
    pub fn into_file(self) -> Vec<u8> {
        self.file
    }

    /// Decodes the global free map (page 1, row 0): bit set = page free or
    /// absent beyond EOF.
    pub fn global_free_map(&self) -> Result<UsageMap, StorageError> {
        self.map_row(MapRef { page: 1, row: 0 })
    }

    /// Decodes the table's used map from its TDEF `used_pages` dword.
    /// Returns the row pointer (for rewrite) plus the decoded map.
    ///
    /// The dword is read directly from the TDEF block head (offset 47,
    /// before the column descriptors) — allocation must not depend on the
    /// table's columns being decodable (type 0x11, BACKLOG L2).
    pub fn table_used_map(&self, tdef_page: u32) -> Result<(MapRef, UsageMap), StorageError> {
        let (_, map_ref) = MapRef::split_tdef_u32(self.tdef_used_pages(tdef_page)?)?;
        let map = self.map_row(map_ref)?;
        Ok((map_ref, map))
    }

    /// Reads the raw `used_pages` dword from the TDEF block head and
    /// validates the page as a plausible TDEF chain head.
    fn tdef_used_pages(&self, tdef_page: u32) -> Result<u32, StorageError> {
        self.ensure_plausible_tdef(tdef_page)?;
        let page = self.header.page_slice(&self.file, tdef_page)?;
        let at = 8 + TDEF_USED_PAGES_AT;
        let raw = page.get(at..at + 4).ok_or_else(|| StorageError::Corrupt {
            context: "usage map".into(),
            detail: format!("TDEF page {tdef_page} is truncated before used_pages"),
        })?;
        Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    /// Pages (numbers, ascending) of type `0x01` owned by `tdef_page` — the
    /// reader's scan heuristic, used by tests to cross-check the maps.
    pub fn data_pages_of<'a>(&'a self, tdef_page: u32) -> impl Iterator<Item = u32> + 'a {
        let file = &self.file;
        let header = &self.header;
        (1..self.header.page_count as u32).filter(move |&n| {
            header.page_slice(file, n).is_ok_and(|page| {
                page_type(page) == PAGE_TYPE_DATA
                    && u32::from_le_bytes([page[4], page[5], page[6], page[7]]) == tdef_page
            })
        })
    }

    /// TDEF chain heads: plausible table definitions that no other TDEF
    /// points at via its next pointer. Chain members are excluded (a chained
    /// TDEF decodes to its head's table); freed TDEF pages re-filled with
    /// the writer's `ff`-pattern (observed on the fixture: pages 322, 707,
    /// 708) fail the [`Jet4Writer::ensure_plausible_tdef`] check.
    pub fn tdef_pages(&self) -> Vec<u32> {
        let mut heads: Vec<u32> = Vec::new();
        let mut linked: Vec<u32> = Vec::new();
        for n in 1..self.header.page_count as u32 {
            let Ok(page) = self.header.page_slice(&self.file, n) else {
                continue;
            };
            if page_type(page) != PAGE_TYPE_TDEF {
                continue;
            }
            heads.push(n);
            let next = u32::from_le_bytes([page[4], page[5], page[6], page[7]]);
            if next != 0 {
                linked.push(next);
            }
        }
        heads.retain(|h| !linked.contains(h) && self.ensure_plausible_tdef(*h).is_ok());
        heads
    }

    /// Validates the fixed TDEF block header enough to trust `used_pages`:
    /// `table_type` must be `0x4E`/`0x53`, `num_cols <= max_cols`, and
    /// `tdef_len` at least the 55-byte fixed header. Freed TDEF pages
    /// (re-filled with `ff 00 ff ff …`) fail this precisely.
    pub(crate) fn ensure_plausible_tdef(&self, tdef_page: u32) -> Result<(), StorageError> {
        let page = self.header.page_slice(&self.file, tdef_page)?;
        if page_type(page) != PAGE_TYPE_TDEF {
            return Err(StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!("page {tdef_page} is not a table definition page"),
            });
        }
        let b = &page[8..];
        let tdef_len = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let table_type = b[32];
        let max_cols = u16::from_le_bytes([b[33], b[34]]);
        let num_cols = u16::from_le_bytes([b[37], b[38]]);
        if tdef_len < 55 || !matches!(table_type, 0x4E | 0x53) || num_cols > max_cols {
            return Err(StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!(
                    "page {tdef_page} carries a TDEF page type but no plausible block \
                     (len {tdef_len}, table type 0x{table_type:02x})"
                ),
            });
        }
        Ok(())
    }

    /// Appends one page at EOF, marks it allocated in the global free map
    /// and in the table's used map, and returns its page number.
    pub fn allocate_page(&mut self, tdef_page: u32) -> Result<u32, StorageError> {
        // Guard: the table definition must exist (validated by the map read).
        let _ = self.tdef_used_pages(tdef_page)?;
        let page_no = self.append_page()?;
        let (map_ref, mut used) = self.table_used_map(tdef_page)?;
        used.set(page_no)?;
        self.rewrite_map_row(map_ref, &used)?;
        Ok(page_no)
    }

    /// Appends one page at EOF, marks it allocated in the global free map,
    /// and returns its page number. No table ownership — the table-map bit
    /// is [`Jet4Writer::allocate_page`]'s addition (TDEF and usage pages
    /// are globally allocated only).
    pub(crate) fn append_page(&mut self) -> Result<u32, StorageError> {
        let page_no = self.header.page_count as u32;
        self.file.resize(self.file.len() + self.header.page_size, 0);
        self.header.page_count += 1;

        // Global free map: grow the window when EOF moves past it, then
        // clear the new page's free bit. Bits above EOF stay set (free).
        let mut global = self.global_free_map()?;
        if !global.covers(page_no) {
            let needed = ((page_no - global.start_page) as usize / 8) + 1;
            global.bitmap.resize(needed, 0xFF);
        }
        global.clear(page_no)?;
        self.rewrite_map_row(MapRef { page: 1, row: 0 }, &global)?;
        Ok(page_no)
    }

    /// Fetches and decodes one usage-map row.
    pub(crate) fn map_row(&self, map_ref: MapRef) -> Result<UsageMap, StorageError> {
        let page = self.header.page_slice(&self.file, map_ref.page)?;
        if page_type(page) != PAGE_TYPE_DATA {
            return Err(StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!("map page {} is not a data page", map_ref.page),
            });
        }
        let decoded = DataPage::decode(page)?;
        let (start, end) = decoded
            .records
            .get(map_ref.row as usize)
            .map(|r| (r.0, r.1))
            .ok_or_else(|| StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!("map page {} holds no row {}", map_ref.page, map_ref.row),
            })?;
        UsageMap::decode_row(&page[start..end])
    }

    /// Rewrites one row of a data page in place, re-packing all rows
    /// back-to-front. Row slot indices (and therefore `(page, row)` map
    /// references) are preserved; offsets and the free-space field are
    /// recomputed (verified formula: `lowest_row_start - (14 + 2*count)`).
    pub(crate) fn rewrite_map_row(
        &mut self,
        map_ref: MapRef,
        map: &UsageMap,
    ) -> Result<(), StorageError> {
        let base = self.header.page_offset(map_ref.page)? as usize;
        let page_size = self.header.page_size;
        let page = &self.file[base..base + page_size];
        let decoded = DataPage::decode(page)?;
        let mut rows: Vec<Vec<u8>> = decoded
            .records
            .iter()
            .map(|&(start, end, _)| page[start..end].to_vec())
            .collect();
        let row = rows
            .get_mut(map_ref.row as usize)
            .ok_or_else(|| StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!("map page {} holds no row {}", map_ref.page, map_ref.row),
            })?;
        *row = map.encode_row();

        let mut end = page_size;
        let mut offsets: Vec<u16> = Vec::with_capacity(rows.len());
        for bytes in &rows {
            let start = end - bytes.len();
            let floor = 14 + 2 * rows.len();
            if start < floor {
                return Err(StorageError::Corrupt {
                    context: "usage map".into(),
                    detail: format!(
                        "map page {} is full: growing row {} needs bytes below the offset table",
                        map_ref.page, map_ref.row
                    ),
                });
            }
            self.file[base + start..base + start + bytes.len()].copy_from_slice(bytes);
            offsets.push(start as u16);
            end = start;
        }
        let free = end - (14 + 2 * rows.len());
        self.file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
        for (i, offset) in offsets.iter().enumerate() {
            self.file[base + 14 + i * 2..base + 16 + i * 2].copy_from_slice(&offset.to_le_bytes());
        }
        Ok(())
    }
}
