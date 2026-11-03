//! Table drops (step 24.2, item M7) — the inverse of `create_table`.
//!
//! A drop removes the `MSysObjects` row, returns every page the table owned
//! (TDAT data pages plus its TDEF and — when no other table references it —
//! its usage page) to the global free map, and overwrites the freed pages
//! with the `0xFF` pattern Access leaves behind (observed on the fixture:
//! freed pages 322, 707, 708). Page selection is deterministic: the owned
//! pages come from the table's used map, the chain from the TDEF `next`
//! pointers, everything else from the catalog.

use std::collections::BTreeSet;

use crate::error::StorageError;
use crate::jet::catalog::CATALOG_TDEF_PAGE;
use crate::jet::reader::Jet4TableReader;
use crate::jet::usage_map::MapRef;
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

/// TDEF block offset of the `used_pages` dword (page header + 47).
const USED_PAGES_AT: usize = 8 + 47;
/// TDEF block offset of the `free_pages` dword.
const FREE_PAGES_AT: usize = 8 + 51;

impl Jet4Writer {
    /// Drops the table rooted at `tdef_page`. The caller resolves the name
    /// through the catalog; the TDEF page is the table's identity.
    pub fn drop_table(&mut self, tdef_page: u32) -> Result<(), StorageError> {
        self.ensure_plausible_tdef(tdef_page)?;
        if tdef_page == CATALOG_TDEF_PAGE {
            return Err(StorageError::Unsupported {
                reason: "the catalog table cannot be dropped".to_owned(),
            });
        }

        // 1. Catalog row of this table (identified by its Id = TDEF page).
        // The writer holds decrypted bytes, so the reader is constructed
        // through the decrypted entry point (no password re-derivation).
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        let catalog = reader.read_table(CATALOG_TDEF_PAGE)?;
        let id_column = catalog
            .schema
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case("Id"))
            .ok_or_else(|| StorageError::Corrupt {
                context: "table drop".into(),
                detail: "MSysObjects has no Id column".to_owned(),
            })?;
        let row_index = catalog
            .rows
            .iter()
            .position(|row| {
                matches!(
                    row.get(id_column),
                    Some(StorageValue::Int(id)) if *id == i64::from(tdef_page)
                )
            })
            .ok_or_else(|| StorageError::Corrupt {
                context: "table drop".into(),
                detail: format!("no MSysObjects row references TDEF page {tdef_page}"),
            })?;
        drop(reader);

        // 2. Every page the table owned plus its TDEF chain and usage page.
        let (used_ref, used) = self.table_used_map(tdef_page)?;
        let (free_ref, _) = self.table_free_map(tdef_page)?;
        let mut freed: BTreeSet<u32> = used.marked_pages().collect();
        let mut chain = vec![tdef_page];
        let mut next = self.page_next(tdef_page)?;
        while next != 0 && next != tdef_page && !chain.contains(&next) {
            chain.push(next);
            next = self.page_next(next)?;
        }
        let referenced = self.map_pages_referenced_except(tdef_page)?;
        for map_ref in [used_ref, free_ref] {
            if !referenced.contains(&map_ref.page) {
                freed.insert(map_ref.page);
            }
        }
        freed.retain(|page| *page > 2 && !chain.contains(page));

        // 3. Remove the catalog row first (a failure leaves the table).
        self.delete_row(CATALOG_TDEF_PAGE, row_index)?;

        // 4. Global free map: freed data/usage pages and the TDEF chain.
        let mut global = self.global_free_map()?;
        for page in freed.iter().chain(chain.iter()) {
            global.set(*page)?;
        }
        self.rewrite_map_row(MapRef { page: 1, row: 0 }, &global)?;

        // 5. Overwrite the freed pages so no stale TDEF head or row bytes
        // survive (deterministic, documented Access-observed pattern).
        for page in freed.iter().chain(chain.iter()) {
            let base = self.header.page_offset(*page)? as usize;
            let size = self.header.page_size;
            self.file[base..base + size].fill(0xFF);
        }
        Ok(())
    }

    /// `next` page of a TDEF chain (`+4` in the page header).
    fn page_next(&self, page: u32) -> Result<u32, StorageError> {
        let slice = self.header.page_slice(&self.file, page)?;
        Ok(u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]))
    }

    /// Usage-map pages referenced by every other table's TDEF dwords — those
    /// must not be freed by a drop.
    fn map_pages_referenced_except(&self, except: u32) -> Result<BTreeSet<u32>, StorageError> {
        let mut referenced = BTreeSet::new();
        for head in self.tdef_pages() {
            if head == except {
                continue;
            }
            let page = self.header.page_slice(&self.file, head)?;
            for at in [USED_PAGES_AT, FREE_PAGES_AT] {
                let raw = u32::from_le_bytes([page[at], page[at + 1], page[at + 2], page[at + 3]]);
                if let Ok((_, map_ref)) = MapRef::split_tdef_u32(raw) {
                    referenced.insert(map_ref.page);
                }
            }
        }
        Ok(referenced)
    }
}
