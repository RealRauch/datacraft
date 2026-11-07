//! Index-leaf maintenance on row mutations (step 49.2, item M63).
//!
//! The engine resolves the catalog and `MSysRelationships` through their
//! physical index B-trees — a row without a leaf entry is invisible (evidence
//! in `docs/FORMAT-JET.md`, "Fresh-create template and the engine's open
//! gates"). The writer therefore keeps every physical leaf of a mutated table
//! in sync: inserts add the row's entry, updates replace it, deletes remove it
//! (entries identify their row by the trailing `(page, row)` pointer, so
//! removals need no key re-encoding).
//!
//! Scope and honesty boundaries:
//!
//! - Every scalar key type is encoded (Byte/Int16/LongInt/Currency/Single/
//!   Double/DateTime/Bool/GUID/Text); Memo/OLE/Binary indexes answer a
//!   `Id`/`ParentId`/`Name` and the relationship names). Any other key type on
//!   an existing physical index answers a precise `Unsupported` instead of
//!   leaving the index silently stale.
//! - Leaves are rebuilt, never split (single-page root leaves); a multi-page
//!   index or an overfull leaf is refused.
//! - The index statistics' row-count dword tracks the distinct key count, the
//!   value real writers store (observed: `MSysACEs` holds 16 entries with 8
//!   distinct `ObjectId` keys and reports 8).

use crate::error::StorageError;
use crate::jet::index_pages::{self, KeyPart, PhysicalIndex, RowPointer, entry_pointer};
use crate::jet::tdef::{JetColumn, JetTableDef, JetType};
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

impl Jet4Writer {
    /// Grows the physical index leaves of `tdef_page` after the row that
    /// landed at `(page, row)`.
    pub(crate) fn maintain_indexes(
        &mut self,
        tdef_page: u32,
        def: &JetTableDef,
        values: &[StorageValue],
        page: u32,
        row: usize,
    ) -> Result<(), StorageError> {
        if index_pages::physical_indexes(def)?.is_empty() {
            return Ok(());
        }
        let pointer = row_pointer(page, row)?;
        for index in index_pages::physical_indexes(def)? {
            if index.root_page == 0 {
                continue;
            }
            let entry = Self::index_entry(def, &index, values, pointer)?;
            self.edit_index_leaf(tdef_page, &index, move |entries| {
                entries.push(entry);
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Drops the leaf entries that address `(page, row)` from every physical
    /// index of the table (a deleted row, or the stale image of an update).
    pub(crate) fn remove_index_entries(
        &mut self,
        tdef_page: u32,
        def: &JetTableDef,
        page: u32,
        row: usize,
    ) -> Result<(), StorageError> {
        let indexes = index_pages::physical_indexes(def)?;
        if indexes.is_empty() {
            return Ok(());
        }
        let pointer = row_pointer(page, row)?;
        for index in indexes {
            if index.root_page == 0 {
                continue;
            }
            self.edit_index_leaf(tdef_page, &index, move |entries| {
                let mut kept = Vec::with_capacity(entries.len());
                for entry in entries.drain(..) {
                    if entry_pointer(&entry)? != pointer {
                        kept.push(entry);
                    }
                }
                *entries = kept;
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Swaps a row's leaf image after an update: the old entry goes, the
    /// new key image takes its place under the same pointer.
    pub(crate) fn replace_index_entries(
        &mut self,
        tdef_page: u32,
        def: &JetTableDef,
        page: u32,
        row: usize,
        values: &[StorageValue],
    ) -> Result<(), StorageError> {
        if index_pages::physical_indexes(def)?.is_empty() {
            return Ok(());
        }
        let pointer = row_pointer(page, row)?;
        for index in index_pages::physical_indexes(def)? {
            if index.root_page == 0 {
                continue;
            }
            let entry = Self::index_entry(def, &index, values, pointer)?;
            self.edit_index_leaf(tdef_page, &index, move |entries| {
                let mut kept = Vec::with_capacity(entries.len() + 1);
                for existing in entries.drain(..) {
                    if entry_pointer(&existing)? != pointer {
                        kept.push(existing);
                    }
                }
                kept.push(entry);
                *entries = kept;
                Ok(())
            })?;
        }
        Ok(())
    }

    /// One key component for an indexed column. The casts mirror
    /// `encode_row`'s storage narrowing, so the key always describes the
    /// value actually written.
    fn key_part<'a>(
        column: &JetColumn,
        value: &'a StorageValue,
    ) -> Result<KeyPart<'a>, StorageError> {
        Ok(match (column.data_type, value) {
            (_, StorageValue::Null) => KeyPart::Null,
            (JetType::Bool, StorageValue::Bit(bit)) => KeyPart::Boolean(*bit),
            (JetType::Byte, StorageValue::Int(number)) => KeyPart::Byte(*number as u8),
            (JetType::Int, StorageValue::Int(number)) => KeyPart::SmallInteger(*number as i16),
            (JetType::LongInt, StorageValue::Int(number)) => KeyPart::Integer(*number),
            (JetType::Currency, StorageValue::Currency(scaled)) => KeyPart::Currency(*scaled),
            (JetType::Float, StorageValue::Float(number)) => KeyPart::Single(*number as f32),
            (JetType::Double, StorageValue::Float(number)) => KeyPart::Double(*number),
            (JetType::DateTime, StorageValue::DateTime(date)) => KeyPart::DateTime(*date),
            (JetType::Guid, StorageValue::Guid(bytes)) => KeyPart::Guid(*bytes),
            (JetType::Text, StorageValue::Text(text)) => KeyPart::Text(text),
            (data_type, other) => {
                return Err(StorageError::Unsupported {
                    reason: format!(
                        "index maintenance cannot encode the key of column {:?}                          ({data_type:?}); it carries {other:?}",
                        column.name
                    ),
                });
            }
        })
    }

    /// Builds one leaf entry for `values` according to the index's columns.
    fn index_entry(
        def: &JetTableDef,
        index: &PhysicalIndex,
        values: &[StorageValue],
        pointer: RowPointer,
    ) -> Result<Vec<u8>, StorageError> {
        let mut parts = Vec::with_capacity(index.columns.len());
        for column_id in &index.columns {
            let position = def
                .columns
                .iter()
                .position(|column| column.column_id == *column_id)
                .ok_or_else(|| StorageError::Corrupt {
                    context: "index maintenance".into(),
                    detail: format!("index references missing column id {column_id}"),
                })?;
            let column = &def.columns[position];
            let value = values.get(position).ok_or_else(|| StorageError::Corrupt {
                context: "index maintenance".into(),
                detail: format!("row carries no value for indexed column {:?}", column.name),
            })?;
            parts.push(Self::key_part(column, value)?);
        }
        index_pages::leaf_entry(&parts, pointer)
    }

    /// Applies `edit` to one leaf root, re-sorts, rewrites the page and
    /// refreshes the index's row-count statistics.
    fn edit_index_leaf(
        &mut self,
        tdef_page: u32,
        index: &PhysicalIndex,
        edit: impl FnOnce(&mut Vec<Vec<u8>>) -> Result<(), StorageError>,
    ) -> Result<(), StorageError> {
        let page_size = self.header.page_size;
        let base = self.header.page_offset(index.root_page)? as usize;
        let page = self
            .file
            .get(base..base + page_size)
            .ok_or_else(|| StorageError::Corrupt {
                context: "index maintenance".into(),
                detail: format!("leaf page {} is beyond the file", index.root_page),
            })?;
        if page[0] != index_pages::PAGE_TYPE_LEAF {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "index root page {} is not a leaf — multi-page index B-trees are \
                     not implemented yet (roadmap M15)",
                    index.root_page
                ),
            });
        }
        let mut leaf = index_pages::decode_leaf(index.root_page, page)?;
        if leaf.parent != tdef_page {
            return Err(StorageError::Corrupt {
                context: "index maintenance".into(),
                detail: format!(
                    "leaf page {} belongs to table definition {} instead of {tdef_page}",
                    index.root_page, leaf.parent
                ),
            });
        }
        edit(&mut leaf.entries)?;
        leaf.entries.sort();
        let encoded = index_pages::encode_leaf(&leaf, page_size)?;
        self.file[base..base + page_size].copy_from_slice(&encoded);

        let distinct = distinct_keys(&leaf.entries);
        let stats = self.header.page_offset(tdef_page)? as usize + 8 + index.stats_offset;
        self.file[stats..stats + 4].copy_from_slice(&(distinct as u32).to_le_bytes());
        Ok(())
    }
}

/// Converts a data-page row slot into the one-byte index pointer form.
fn row_pointer(page: u32, row: usize) -> Result<RowPointer, StorageError> {
    let row = u8::try_from(row).map_err(|_| StorageError::Unsupported {
        reason: format!("index maintenance: row slot {row} does not fit the one-byte pointer"),
    })?;
    Ok(RowPointer { page, row })
}

/// Number of distinct keys in a leaf (entries minus their row pointers).
fn distinct_keys(entries: &[Vec<u8>]) -> usize {
    let mut keys: Vec<&[u8]> = entries
        .iter()
        .map(|entry| &entry[..entry.len().saturating_sub(4)])
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys.len()
}

#[cfg(test)]
mod tests;
