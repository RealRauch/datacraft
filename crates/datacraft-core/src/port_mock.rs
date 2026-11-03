//! In-memory storage port for headless tests (split from `port.rs` to keep
//! every file under the size gate).

use std::collections::HashMap;

use crate::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, StoragePort, window_slice,
};
use crate::schema::{FieldValue, TableDef};

/// In-memory backend for headless tests and early UI wiring (mirrors
/// storage's `InMemoryEngine`; no filesystem involvement).
#[derive(Debug, Default)]
pub struct MockPort {
    tables: Vec<TableDef>,
    rows: HashMap<String, Vec<Vec<FieldValue>>>,
    relationships: Vec<Relationship>,
    macros: Vec<crate::macro_def::MacroDef>,
    vba: Option<crate::vba::VbaProject>,
    vba_error: Option<String>,
    /// When set, `native_queries` fails with this reason (M10 tests).
    native_queries_error: Option<String>,
    /// Native query inventory (M3 surface; set by tests).
    native_queries: Vec<crate::port::NativeQuery>,
    opened: Option<OpenedDatabase>,
    open_fail: Option<String>,
    write_fail: Option<String>,
    read_fail: Option<String>,
    /// When set, `open` requires this database password (M5 tests).
    password: Option<String>,
    /// Overrides the reported capabilities (phase 30, M17: restricted-
    /// driver simulations). `None` reports the honest mock default.
    reported_capabilities: Option<DriverCapabilities>,
    /// Number of successful `open` calls (observability for tests).
    pub open_calls: usize,
    /// Number of `close` calls (observability for tests).
    pub close_calls: usize,
    /// Save-As calls recorded for core tests (path, password).
    pub save_as_calls: Vec<(std::path::PathBuf, Option<String>)>,
    /// Password-change calls recorded for core tests.
    pub change_password_calls: Vec<Option<String>>,
}

mod alter;

impl MockPort {
    /// Creates an empty mock that opens as `"mock"` (Jet4).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a mock pre-seeded with table definitions.
    pub fn with_tables(tables: Vec<TableDef>) -> Self {
        Self {
            tables,
            ..Self::default()
        }
    }

    /// Builder: attaches row data for a table (case-insensitive reads).
    pub fn with_row_data(
        &mut self,
        table: impl Into<String>,
        rows: Vec<Vec<FieldValue>>,
    ) -> &mut Self {
        self.rows.insert(table.into(), rows);
        self
    }

    /// Builder: attaches referential relationships.
    pub fn with_relationships(&mut self, relationships: Vec<Relationship>) -> &mut Self {
        self.relationships = relationships;
        self
    }

    /// Builder: attaches the named-macro inventory (step 17.1, L4).
    pub fn with_macros(&mut self, macros: Vec<crate::macro_def::MacroDef>) -> &mut Self {
        self.macros = macros;
        self
    }

    /// Builder: attaches an extracted VBA project (step 17.2, L5).
    pub fn with_vba(&mut self, project: crate::vba::VbaProject) -> &mut Self {
        self.vba = Some(project);
        self
    }

    /// Builder: makes the VBA extraction fail with `reason` (observable
    /// degradation; the database still opens).
    pub fn with_vba_error(&mut self, reason: impl Into<String>) -> &mut Self {
        self.vba_error = Some(reason.into());
        self
    }

    /// Builder: makes the native-query decode fail with `reason` (M10).
    pub fn with_native_queries_error(&mut self, reason: impl Into<String>) -> &mut Self {
        self.native_queries_error = Some(reason.into());
        self
    }

    /// Builder: attaches a native query inventory (M3).
    pub fn with_native_queries(&mut self, queries: Vec<crate::port::NativeQuery>) -> &mut Self {
        self.native_queries = queries;
        self
    }

    /// Creates a mock whose `open` always fails with `detail`.
    pub fn failing_open(detail: impl Into<String>) -> Self {
        Self {
            open_fail: Some(detail.into()),
            ..Self::default()
        }
    }

    /// Creates a mock whose write operations always fail with `detail`.
    pub fn failing_writes(detail: impl Into<String>) -> Self {
        Self {
            write_fail: Some(detail.into()),
            ..Self::default()
        }
    }

    /// Builder: makes every subsequent write fail with `detail`.
    pub fn with_write_failure(&mut self, detail: impl Into<String>) -> &mut Self {
        self.write_fail = Some(detail.into());
        self
    }

    /// Builder: makes every subsequent row read fail with `detail`
    /// (materialization-failure tests).
    pub fn with_read_failure(&mut self, detail: impl Into<String>) -> &mut Self {
        self.read_fail = Some(detail.into());
        self
    }

    /// Builder: requires `password` on `open` (database-password flow, M5).
    pub fn with_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.password = Some(password.into());
        self
    }

    /// Builder: overrides the reported capabilities (phase 30, M17 —
    /// simulated restricted drivers).
    pub fn with_capabilities(&mut self, capabilities: DriverCapabilities) -> &mut Self {
        self.reported_capabilities = Some(capabilities);
        self
    }

    /// Resolves a table definition case-insensitively.
    fn table_def(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }
}

/// Out-of-range error for a single addressed row.
fn out_of_range(table: &str, row: usize, row_count: usize) -> PortError {
    PortError::OutOfRange {
        table: table.to_owned(),
        start: row,
        len: 1,
        row_count,
    }
}

impl StoragePort for MockPort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("mock")
    }

    /// The honest mock default: everything DataCraft's commands can
    /// drive except atomic batches (the trait default applies batches
    /// one by one) and passwords (the mock ignores them).
    fn capabilities(&self) -> DriverCapabilities {
        self.reported_capabilities.unwrap_or_else(|| {
            let mut caps = DriverCapabilities::full();
            caps.passwords = false;
            caps
        })
    }

    fn open(
        &mut self,
        path: &std::path::Path,
        password: Option<&str>,
    ) -> Result<OpenedDatabase, PortError> {
        if let Some(detail) = &self.open_fail {
            return Err(PortError::OpenFailed {
                detail: detail.clone(),
            });
        }
        if let Some(expected) = &self.password {
            match password {
                None => {
                    return Err(PortError::PasswordRequired {
                        path: path.display().to_string(),
                    });
                }
                Some(given) if given != expected => {
                    return Err(PortError::WrongPassword {
                        path: path.display().to_string(),
                    });
                }
                Some(_) => {}
            }
        }
        self.open_calls += 1;
        let name = path
            .file_stem()
            .map_or_else(|| "mock".to_owned(), |s| s.to_string_lossy().into_owned());
        let opened = self
            .opened
            .get_or_insert(OpenedDatabase {
                name,
                format: DatabaseFormat::Jet4,
            })
            .clone();
        Ok(opened)
    }

    fn close(&mut self) {
        self.close_calls += 1;
        self.opened = None;
    }

    fn create_database(
        &mut self,
        path: &std::path::Path,
        format: DatabaseFormat,
    ) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let _ = (path, format);
        Ok(())
    }

    fn save_as(&mut self, path: &std::path::Path, password: Option<&str>) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        self.save_as_calls
            .push((path.to_path_buf(), password.map(str::to_owned)));
        Ok(())
    }

    fn change_password(&mut self, new_password: Option<&str>) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        self.change_password_calls
            .push(new_password.map(str::to_owned));
        Ok(())
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        self.tables.push(definition.clone());
        Ok(())
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let pos = self
            .tables
            .iter()
            .position(|t| t.name.to_lowercase() == name.to_lowercase())
            .ok_or_else(|| PortError::TableNotFound(name.to_owned()))?;
        Ok(self.tables.remove(pos).name)
    }

    fn alter_table(
        &mut self,
        table: &str,
        change: crate::port::SchemaChange,
    ) -> Result<(), PortError> {
        alter::alter(self, table, change)
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        if self.opened.is_none() {
            return Err(PortError::Unsupported {
                reason: "no database is open".to_owned(),
            });
        }
        if let Some(detail) = &self.read_fail {
            return Err(PortError::Unsupported {
                reason: detail.clone(),
            });
        }
        // A catalogued table without a row entry is an empty table.
        let def = self.table_def(table)?;
        let empty: Vec<Vec<FieldValue>> = Vec::new();
        let rows = self
            .rows
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(&def.name))
            .map(|(_, rows)| rows)
            .unwrap_or(&empty);
        let (start, end) = window_slice(table, range, rows.len())?;
        Ok(rows[start..end].to_vec())
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let Some(pos) = self
            .tables
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
        let def = self.tables[pos].clone();
        if values.len() != def.fields.len() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "insert into {:?}: expected {} values, found {}",
                    def.name,
                    def.fields.len(),
                    values.len()
                ),
            });
        }
        self.rows.entry(def.name).or_default().push(values.to_vec());
        Ok(())
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let def = self.table_def(table)?.clone();
        if values.len() != def.fields.len() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "update {:?}: expected {} values, found {}",
                    def.name,
                    def.fields.len(),
                    values.len()
                ),
            });
        }
        let rows = self
            .rows
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(&def.name))
            .map(|(_, rows)| rows);
        let Some(rows) = rows else {
            return Err(out_of_range(table, row, 0));
        };
        let Some(target) = rows.get_mut(row) else {
            return Err(out_of_range(table, row, rows.len()));
        };
        *target = values.to_vec();
        Ok(())
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let def = self.table_def(table)?.clone();
        let rows = self
            .rows
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(&def.name))
            .map(|(_, rows)| rows);
        let Some(rows) = rows else {
            return Err(out_of_range(table, row, 0));
        };
        if row >= rows.len() {
            return Err(out_of_range(table, row, rows.len()));
        }
        rows.remove(row);
        Ok(())
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(self.relationships.clone())
    }

    fn create_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        if self.relationships.iter().any(|entry| entry == relationship) {
            return Err(PortError::WriteFailed {
                detail: format!("duplicate relationship {relationship:?}"),
            });
        }
        self.relationships.push(relationship.clone());
        Ok(())
    }

    fn delete_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        if let Some(detail) = &self.write_fail {
            return Err(PortError::WriteFailed {
                detail: detail.clone(),
            });
        }
        let Some(position) = self
            .relationships
            .iter()
            .position(|entry| entry == relationship)
        else {
            return Err(PortError::WriteFailed {
                detail: format!("relationship {relationship:?} not found"),
            });
        };
        self.relationships.remove(position);
        Ok(())
    }

    fn macros(&self) -> Result<Vec<crate::macro_def::MacroDef>, PortError> {
        Ok(self.macros.clone())
    }

    fn native_queries(&self) -> Result<Vec<crate::port::NativeQuery>, PortError> {
        if let Some(reason) = &self.native_queries_error {
            return Err(PortError::Unsupported {
                reason: reason.clone(),
            });
        }
        Ok(self.native_queries.clone())
    }

    fn vba_project(&self) -> Result<Option<crate::vba::VbaProject>, PortError> {
        if let Some(reason) = &self.vba_error {
            return Err(PortError::Unsupported {
                reason: reason.clone(),
            });
        }
        Ok(self.vba.clone())
    }
}

#[cfg(test)]
mod tests;
