//! Row update and delete (step 11.10, item H40) — the third write half of
//! the Jet4 writer, split from table creation/insertion.
//!
//! Deletes follow the Access convention: the row's offset-table entry gets
//! the `0x8000` deleted flag and the bytes stay in place (space is
//! reclaimed by a later compaction, never by the delete). Updates take the
//! in-place path when the encoded row keeps its length; otherwise the page
//! is repacked back-to-front with every slot's flags preserved. Rows that
//! live behind an overflow pointer or an external LVAL chain answer a
//! precise `Unsupported` (unqualified row forms).

use crate::error::StorageError;
use crate::jet::data_page::DataPage;
use crate::jet::encode::encode_row;
use crate::jet::page::{PAGE_TYPE_DATA, page_type};
use crate::jet::row::has_external_long_value;
use crate::jet::tdef::JetTableDef;
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

/// Deleted-row flag of a data-page offset entry.
const DELETED: u16 = 0x8000;
/// Overflow flag of a data-page offset entry (holds a 4-byte row pointer).
const OVERFLOW: u16 = 0x4000;

impl Jet4Writer {
    /// Locates the `index`-th live row of a table: `(page, slot)`.
    ///
    /// Live rows are the slots without the `0x8000` flag, in page/slot
    /// order — exactly the order the reader returns them.
    pub fn row_location(&self, tdef_page: u32, index: usize) -> Result<(u32, usize), StorageError> {
        self.ensure_plausible_tdef(tdef_page)?;
        let mut live = 0usize;
        for page_no in 1..self.header.page_count as u32 {
            let page = self.header.page_slice(&self.file, page_no)?;
            if page_type(page) != PAGE_TYPE_DATA {
                continue;
            }
            let decoded = DataPage::decode(page)?;
            if decoded.owner != tdef_page {
                continue;
            }
            for (slot, &(_, _, raw)) in decoded.records.iter().enumerate() {
                if raw & DELETED != 0 {
                    continue;
                }
                if live == index {
                    return Ok((page_no, slot));
                }
                live += 1;
            }
        }
        Err(StorageError::OutOfRange {
            table: format!("tdef_{tdef_page:04}"),
            start: index,
            len: 1,
            row_count: live,
        })
    }

    /// Marks the `index`-th live row deleted (offset flag `0x8000`). The
    /// row's bytes stay in place — Access reclaims them on compaction.
    pub fn delete_row(&mut self, tdef_page: u32, index: usize) -> Result<(), StorageError> {
        let (page_no, slot) = self.row_location(tdef_page, index)?;
        let base = self.header.page_offset(page_no)? as usize;
        let at = base + 14 + slot * 2;
        let raw = u16::from_le_bytes([self.file[at], self.file[at + 1]]);
        self.file[at..at + 2].copy_from_slice(&(raw | DELETED).to_le_bytes());
        self.adjust_num_rows(tdef_page, -1)?;
        Ok(())
    }

    /// Replaces the `index`-th live row. Same-length encodings overwrite in
    /// place; a different length repacks the page (bounds-checked).
    pub fn update_row(
        &mut self,
        tdef_page: u32,
        index: usize,
        values: &[StorageValue],
    ) -> Result<(), StorageError> {
        let def = JetTableDef::decode(&self.file, tdef_page)?;
        let (page_no, slot) = self.row_location(tdef_page, index)?;
        let base = self.header.page_offset(page_no)? as usize;
        let page_size = self.header.page_size;
        let decoded = DataPage::decode(&self.file[base..base + page_size])?;
        let (start, end, raw) = decoded.records[slot];
        if raw & OVERFLOW != 0 {
            return Err(StorageError::Unsupported {
                reason: "row lives behind an overflow pointer — updating it is not \
                         qualified yet (BACKLOG H40)"
                    .to_owned(),
            });
        }
        if has_external_long_value(&self.file[base + start..base + end], &def)? {
            return Err(StorageError::Unsupported {
                reason: "row stores a memo/OLE value in an external LVAL chain — \
                         updating it is not qualified yet (BACKLOG H40)"
                    .to_owned(),
            });
        }
        let encoded = encode_row(values, &def)?;
        if encoded.len() == end - start {
            self.file[base + start..base + end].copy_from_slice(&encoded);
            return Ok(());
        }

        let mut rows: Vec<(Vec<u8>, u16)> = decoded
            .records
            .iter()
            .map(|&(start, end, raw)| (self.file[base + start..base + end].to_vec(), raw))
            .collect();
        rows[slot].0 = encoded;
        self.repack_data_page(page_no, &rows)?;
        self.update_free_bit(tdef_page, page_no)?;
        Ok(())
    }

    /// Rewrites every slot of a data page back-to-front (offsets descend),
    /// preserving each slot's `0x8000`/`0x4000` flags. The layout is
    /// validated before any byte is written.
    fn repack_data_page(
        &mut self,
        page_no: u32,
        rows: &[(Vec<u8>, u16)],
    ) -> Result<(), StorageError> {
        let base = self.header.page_offset(page_no)? as usize;
        let page_size = self.header.page_size;
        let floor = 14 + 2 * rows.len();
        let mut end = page_size;
        let mut offsets = Vec::with_capacity(rows.len());
        for (bytes, raw) in rows {
            let Some(start) = end.checked_sub(bytes.len()) else {
                return Err(page_full(page_no, bytes.len()));
            };
            if start < floor {
                return Err(page_full(page_no, bytes.len()));
            }
            offsets.push((start as u16) | (raw & (DELETED | OVERFLOW)));
            end = start;
        }
        let mut cursor = page_size;
        for (bytes, _) in rows {
            cursor -= bytes.len();
            self.file[base + cursor..base + cursor + bytes.len()].copy_from_slice(bytes);
        }
        for (slot, offset) in offsets.iter().enumerate() {
            self.file[base + 14 + slot * 2..base + 16 + slot * 2]
                .copy_from_slice(&offset.to_le_bytes());
        }
        self.file[base + 2..base + 4].copy_from_slice(&((end - floor) as u16).to_le_bytes());
        Ok(())
    }
}

fn page_full(page_no: u32, row_len: usize) -> StorageError {
    StorageError::Unsupported {
        reason: format!(
            "the updated row ({row_len} bytes) does not fit page {page_no} and page \
             compaction into a new page is not qualified yet (BACKLOG H40)"
        ),
    }
}
