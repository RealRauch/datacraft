//! Table creation and row insertion (step 10.2, item H9) — the write
//! half of the Jet4 writer, split from the allocation core in `writer.rs`.
//!
//! Catalog recipe (verified against both fixture writers, see
//! `docs/FORMAT-JET.md` "Catalog insert recipe"): a new table is an
//! MSysObjects row `Id = tdef page, ParentId = 0x0F000001 ("Tables"
//! container), Type = 1, Flags = 0, Owner = e5 a3` with NULL `LvProp`
//! (Access accepts NULL LvProp on catalog rows — system rows carry it).
//!
//! Page economics: the table's usage page carries two rows — used map
//! (row 0) and free map (row 1). Free-map polarity: bit set = page has
//! room (observed bracket on the fixture: 75 bytes free → unmarked,
//! 966 → marked); the writer marks conservatively at ≥ 512 bytes.

use crate::error::StorageError;
use crate::jet::data_page::DataPage;
use crate::jet::encode::{
    build_table_def, encode_row, encode_tdef, now_as_ole_date, pack_map_dword,
};
use crate::jet::page::{PAGE_TYPE_DATA, PAGE_TYPE_TDEF};
use crate::jet::tdef::JetTableDef;
use crate::jet::usage_map::UsageMap;
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

use super::tdef::JetColumn;

/// `MSysObjects.ParentId` of the "Tables" container (fixture: 0x0F000001).
const TABLES_CONTAINER_ID: i64 = 0x0F00_0001;
/// Owner bytes of user-created table rows (fixture: both writers agree).
const USER_TABLE_OWNER: [u8; 2] = [0xE5, 0xA3];
/// Free-space reserve: pages below this are unmarked in the free map.
const FREE_MAP_RESERVE: usize = 512;
/// `MSysObjects` TDEF page (format constant, see `catalog.rs`).
const CATALOG_TDEF: u32 = 2;

impl Jet4Writer {
    /// Reads the table's free map (`free_pages` dword): bit set = page has
    /// room for another row.
    pub fn table_free_map(
        &self,
        tdef_page: u32,
    ) -> Result<(super::usage_map::MapRef, UsageMap), StorageError> {
        self.ensure_plausible_tdef(tdef_page)?;
        let page = self.header.page_slice(&self.file, tdef_page)?;
        let at = 8 + 51; // TDEF block offset of free_pages
        let raw = page.get(at..at + 4).ok_or_else(|| StorageError::Corrupt {
            context: "usage map".into(),
            detail: format!("TDEF page {tdef_page} is truncated before free_pages"),
        })?;
        let (_, map_ref) = super::usage_map::MapRef::split_tdef_u32(u32::from_le_bytes([
            raw[0], raw[1], raw[2], raw[3],
        ]))?;
        let map = self.map_row(map_ref)?;
        Ok((map_ref, map))
    }

    /// Creates a table: dedicated usage page, TDEF page, catalog row.
    /// Returns the new table's TDEF page number (its identity at every
    /// boundary — the catalog `Id`).
    pub fn create_table(&mut self, name: &str, columns: &[JetColumn]) -> Result<u32, StorageError> {
        // 1. Usage page (owner 0) with empty used/free rows.
        let usage_page = self.append_page()?;
        let empty = UsageMap {
            start_page: 0,
            bitmap: vec![0u8; 8],
        }
        .encode_row();
        self.write_fresh_data_page(usage_page, 0, &[empty.clone(), empty])?;

        // 2. TDEF page wired to the usage rows.
        let tdef_page = self.append_page()?;
        let def = build_table_def(
            0x4E,
            0,
            columns,
            pack_map_dword(0x00, usage_page, 0),
            pack_map_dword(0x00, usage_page, 1),
        );
        self.write_fresh_tdef_page(tdef_page, &encode_tdef(&def))?;

        // 3. Catalog row (append into MSysObjects).
        let now = now_as_ole_date();
        let catalog_def = JetTableDef::decode(&self.file, CATALOG_TDEF)?;
        let values = Self::catalog_row(&catalog_def, tdef_page, name, now);
        self.insert_row(CATALOG_TDEF, &values)?;
        Ok(tdef_page)
    }

    /// MSysObjects column values for a new user table, driven by the
    /// catalog's actual definition (column-name keyed — robust against
    /// catalogs with more or fewer columns; unknown columns stay NULL).
    fn catalog_row(def: &JetTableDef, tdef_page: u32, name: &str, now: f64) -> Vec<StorageValue> {
        def.columns
            .iter()
            .map(|col| match col.name.as_str() {
                "Id" => StorageValue::Int(i64::from(tdef_page)),
                "ParentId" => StorageValue::Int(TABLES_CONTAINER_ID),
                "Name" => StorageValue::Text(name.to_owned()),
                "Type" => StorageValue::Int(1),
                "DateCreate" | "DateUpdate" => StorageValue::DateTime(now),
                "Owner" => StorageValue::Binary(USER_TABLE_OWNER.to_vec()),
                "Flags" => StorageValue::Int(0),
                _ => StorageValue::Null, // LvProp etc. (see module docs)
            })
            .collect()
    }

    /// Appends one row to the table rooted at `tdef_page`, reusing a page
    /// with room (free map) or allocating a fresh one.
    pub fn insert_row(
        &mut self,
        tdef_page: u32,
        values: &[StorageValue],
    ) -> Result<(), StorageError> {
        let def = JetTableDef::decode(&self.file, tdef_page)?;
        let row = encode_row(values, &def)?;
        let (_, free) = self.table_free_map(tdef_page)?;
        let target = free
            .marked_pages()
            .find(|&page| self.page_has_room(page, row.len()))
            .map_or_else(|| self.allocate_data_page(tdef_page), Ok)?;
        self.append_row_to_page(target, &row)?;
        self.bump_num_rows(tdef_page)?;
        self.update_free_bit(tdef_page, target)?;
        Ok(())
    }

    /// Whether `page` can hold one more row of `row_len` bytes. The offset
    /// table is sorted by descending offset — the physically lowest row is
    /// the table's LAST entry.
    fn page_has_room(&self, page_no: u32, row_len: usize) -> bool {
        let Ok(page) = self.header.page_slice(&self.file, page_no) else {
            return false;
        };
        let Ok(decoded) = DataPage::decode(page) else {
            return false;
        };
        let lowest = decoded
            .records
            .last()
            .map_or(self.header.page_size, |r| r.0);
        lowest >= 14 + 2 * (decoded.records.len() + 1) + row_len
    }

    /// Allocates a fresh data page for the table (owned, empty, free).
    fn allocate_data_page(&mut self, tdef_page: u32) -> Result<u32, StorageError> {
        let page_no = self.allocate_page(tdef_page)?;
        self.write_fresh_data_page(page_no, tdef_page, &[])?;
        Ok(page_no)
    }

    /// Appends raw row bytes to a data page: the row lands between the
    /// offset table and the physically lowest row; its offset extends the
    /// table (which is sorted by descending offset).
    fn append_row_to_page(&mut self, page_no: u32, row: &[u8]) -> Result<(), StorageError> {
        let base = self.header.page_offset(page_no)? as usize;
        let page_size = self.header.page_size;
        let decoded = DataPage::decode(&self.file[base..base + page_size])?;
        let count = decoded.records.len();
        let lowest = decoded.records.last().map_or(page_size, |r| r.0);
        let floor = 14 + 2 * (count + 1);
        let start = lowest - row.len();
        if start < floor {
            return Err(StorageError::Corrupt {
                context: "row insert".into(),
                detail: format!("page {page_no} cannot hold {} more bytes", row.len()),
            });
        }
        self.file[base + start..base + start + row.len()].copy_from_slice(row);
        self.file[base + 14 + count * 2..base + 16 + count * 2]
            .copy_from_slice(&(start as u16).to_le_bytes());
        self.file[base + 12..base + 14].copy_from_slice(&((count + 1) as u16).to_le_bytes());
        let free = start - floor;
        self.file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
        Ok(())
    }

    /// `num_rows += 1` in the TDEF block head (in place, same size).
    fn bump_num_rows(&mut self, tdef_page: u32) -> Result<(), StorageError> {
        self.adjust_num_rows(tdef_page, 1)
    }

    /// Applies a signed delta to `num_rows` in the TDEF block head
    /// (saturating at zero); inserts bump, deletes decrement.
    pub(crate) fn adjust_num_rows(
        &mut self,
        tdef_page: u32,
        delta: i64,
    ) -> Result<(), StorageError> {
        let base = self.header.page_offset(tdef_page)? as usize;
        let at = base + 8 + 8; // page header + block offset of num_rows
        let rows = u32::from_le_bytes([
            self.file[at],
            self.file[at + 1],
            self.file[at + 2],
            self.file[at + 3],
        ]);
        let updated = if delta >= 0 {
            rows.saturating_add(delta as u32)
        } else {
            rows.saturating_sub(delta.unsigned_abs() as u32)
        };
        self.file[at..at + 4].copy_from_slice(&updated.to_le_bytes());
        Ok(())
    }

    /// Marks `page_no` in the free map while it holds ≥ `FREE_MAP_RESERVE`
    /// bytes, clears the bit below (the hint must never over-promise).
    pub(crate) fn update_free_bit(
        &mut self,
        tdef_page: u32,
        page_no: u32,
    ) -> Result<(), StorageError> {
        let base = self.header.page_offset(page_no)? as usize;
        let free_space = u16::from_le_bytes([self.file[base + 2], self.file[base + 3]]) as usize;
        let (map_ref, mut map) = self.table_free_map(tdef_page)?;
        if free_space >= FREE_MAP_RESERVE {
            map.set(page_no)?;
        } else {
            map.clear(page_no)?;
        }
        self.rewrite_map_row(map_ref, &map)?;
        Ok(())
    }

    /// Writes a fresh data page (empty or with pre-encoded rows).
    fn write_fresh_data_page(
        &mut self,
        page_no: u32,
        owner: u32,
        rows: &[Vec<u8>],
    ) -> Result<(), StorageError> {
        let base = self.header.page_offset(page_no)? as usize;
        self.file[base] = PAGE_TYPE_DATA;
        self.file[base + 1] = 0x01;
        self.file[base + 4..base + 8].copy_from_slice(&owner.to_le_bytes());
        self.file[base + 12..base + 14].copy_from_slice(&(rows.len() as u16).to_le_bytes());
        let mut end = self.header.page_size;
        for (i, row) in rows.iter().enumerate() {
            let start = end - row.len();
            self.file[base + start..base + start + row.len()].copy_from_slice(row);
            self.file[base + 14 + i * 2..base + 16 + i * 2]
                .copy_from_slice(&(start as u16).to_le_bytes());
            end = start;
        }
        let free = end - (14 + 2 * rows.len());
        self.file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
        Ok(())
    }

    /// Writes a TDEF page around a pre-encoded block.
    fn write_fresh_tdef_page(&mut self, page_no: u32, block: &[u8]) -> Result<(), StorageError> {
        let base = self.header.page_offset(page_no)? as usize;
        self.file[base] = PAGE_TYPE_TDEF;
        self.file[base + 1] = 0x01;
        let free = self.header.page_size as u32 - 8 - block.len() as u32;
        self.file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
        self.file[base + 4..base + 8].copy_from_slice(&0u32.to_le_bytes()); // no chain
        self.file[base + 8..base + 8 + block.len()].copy_from_slice(block);
        Ok(())
    }
}
