//! MSysObjects catalog enumeration (step 2.4, item H6).
//!
//! The catalog's own TDEF is always page 2. Names attach to tables only when
//! the row's `Id` lands on an independently scanned TDEF page — page reuse
//! can stale the Id, so a mismatch means "addressable by page, without name"
//! (evidence documented in `docs/FORMAT-JET.md`).

use crate::error::StorageError;
use crate::jet::page::{PAGE_TYPE_TDEF, page_type};
use crate::jet::reader::{Jet4TableReader, JetTableData};
use crate::jet::tdef::JetTableDef;
use crate::types::StorageValue;

/// Object type constant for tables in MSysObjects rows.
pub const OBJECT_TYPE_TABLE: i64 = 1;

/// One catalog entry.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    /// Object name (table/query/form/...).
    pub name: String,
    /// MSysObjects `Type` value (1 = table).
    pub object_type: i64,
    /// MSysObjects `Flags` value (system-table bits live here).
    pub flags: i64,
    /// TDEF page, `Some` only when the row's `Id` matches an independently
    /// scanned TDEF page; `None` marks a stale/reused Id.
    pub tdef_page: Option<u32>,
}

impl Jet4TableReader {
    /// Pages whose first byte marks them as TDEF pages, ascending.
    pub fn scan_tdef_pages(&self) -> Vec<u32> {
        (1..self.header.page_count as u32)
            .filter(|page| {
                self.header
                    .page_slice(&self.file, *page)
                    .map(|slice| page_type(slice) == PAGE_TYPE_TDEF)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Reads the MSysObjects catalog (page 2) and cross-references table
    /// entries against the scanned TDEF pages.
    pub fn catalog(&self) -> Result<Vec<CatalogEntry>, StorageError> {
        let tdefs = self.scan_tdef_pages();
        let def = JetTableDef::decode(&self.file, CATALOG_TDEF_PAGE)?;
        let index_of = |name: &str| -> Result<usize, StorageError> {
            def.columns
                .iter()
                .position(|c| c.name == name)
                .ok_or_else(|| StorageError::Corrupt {
                    context: "catalog".into(),
                    detail: format!("catalog lacks the {name} column"),
                })
        };
        let i_id = index_of("Id")?;
        let i_name = index_of("Name")?;
        let i_type = index_of("Type")?;
        let i_flags = index_of("Flags")?;

        let table = self.read_table(CATALOG_TDEF_PAGE)?;
        let mut entries = Vec::with_capacity(table.rows.len());
        for row in &table.rows {
            // Rows without a name/type (e.g. relationship or property rows)
            // are not nameable objects — skip them.
            let (Some(id), Some(name), Some(object_type), Some(flags)) = (
                as_opt_int(&row[i_id]),
                as_opt_text(&row[i_name]),
                as_opt_int(&row[i_type]),
                as_opt_int(&row[i_flags]),
            ) else {
                continue;
            };
            let tdef_page = if object_type == OBJECT_TYPE_TABLE && tdefs.contains(&(id as u32)) {
                Some(id as u32)
            } else {
                None
            };
            entries.push(CatalogEntry {
                name,
                object_type,
                flags,
                tdef_page,
            });
        }
        Ok(entries)
    }

    /// Names of user tables whose TDEF page could be resolved.
    pub fn table_names(&self) -> Result<Vec<String>, StorageError> {
        Ok(self
            .catalog()?
            .into_iter()
            .filter(|e| e.object_type == OBJECT_TYPE_TABLE && e.tdef_page.is_some())
            .map(|e| e.name)
            .collect())
    }

    /// Opens a table by catalog name; the schema carries the real name.
    pub fn open_table(&self, name: &str) -> Result<JetTableData, StorageError> {
        let entry = self
            .catalog()?
            .into_iter()
            .find(|e| e.object_type == OBJECT_TYPE_TABLE && e.tdef_page.is_some() && e.name == name)
            .ok_or_else(|| StorageError::TableNotFound(name.to_owned()))?;
        let mut data = self.read_table(entry.tdef_page.expect("checked above"))?;
        data.schema.name = entry.name;
        Ok(data)
    }
}

/// Integer variant accessor; `None` for NULL/non-integer values.
fn as_opt_int(v: &StorageValue) -> Option<i64> {
    match v {
        StorageValue::Int(n) => Some(*n),
        _ => None,
    }
}

/// Text variant accessor; `None` for NULL/non-text values.
fn as_opt_text(v: &StorageValue) -> Option<String> {
    match v {
        StorageValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}

/// The MSysObjects table definition page (format constant).
pub const CATALOG_TDEF_PAGE: u32 = 2;
