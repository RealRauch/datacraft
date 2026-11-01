//! In-memory storage port for headless tests (split from `port.rs` to keep
//! every file under the size gate).

use std::collections::HashMap;

use crate::port::{
    DatabaseFormat, OpenedDatabase, PortError, Relationship, RowRange, StoragePort, window_slice,
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
    opened: Option<OpenedDatabase>,
    open_fail: Option<String>,
    write_fail: Option<String>,
    read_fail: Option<String>,
    /// When set, `open` requires this database password (M5 tests).
    password: Option<String>,
    /// Number of successful `open` calls (observability for tests).
    pub open_calls: usize,
    /// Number of `close` calls (observability for tests).
    pub close_calls: usize,
}

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

    fn macros(&self) -> Result<Vec<crate::macro_def::MacroDef>, PortError> {
        Ok(self.macros.clone())
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
mod tests {
    use super::*;
    use crate::schema::{FieldDef, FieldKind};

    fn table() -> TableDef {
        TableDef::new(
            "T",
            vec![FieldDef::new(
                "n",
                FieldKind::Number(crate::schema::NumericKind::LongInteger),
            )],
        )
    }

    /// Every write path reports a configured failure before touching state.
    #[test]
    fn mock_write_failures_cover_all_write_operations() {
        let mut port = MockPort::failing_writes("disk full");
        assert!(matches!(
            port.create_table(&table()),
            Err(PortError::WriteFailed { .. })
        ));
        assert!(matches!(
            port.drop_table("T"),
            Err(PortError::WriteFailed { .. })
        ));
        assert!(matches!(
            port.insert_row("T", &[FieldValue::Integer(1)]),
            Err(PortError::WriteFailed { .. })
        ));
        assert!(matches!(
            port.update_row("T", 0, &[FieldValue::Integer(1)]),
            Err(PortError::WriteFailed { .. })
        ));
        assert!(matches!(
            port.delete_row("T", 0),
            Err(PortError::WriteFailed { .. })
        ));
    }

    /// The mock applies row operations by live-row index and reports
    /// precise bounds and arity failures.
    #[test]
    fn mock_row_operations_cover_bounds_and_arity() {
        let mut port = MockPort::with_tables(vec![table()]);
        port.with_row_data("T", vec![vec![FieldValue::Integer(1)]]);
        port.update_row("t", 0, &[FieldValue::Integer(2)])
            .expect("case-insensitive update");
        port.open(std::path::Path::new("memory.mock"), None)
            .expect("open the mock");
        assert_eq!(
            port.read_rows("T", RowRange { start: 0, len: 1 })
                .expect("row stored"),
            vec![vec![FieldValue::Integer(2)]]
        );

        assert!(matches!(
            port.update_row("T", 0, &[]),
            Err(PortError::WriteFailed { .. })
        ));
        assert!(matches!(
            port.update_row("T", 3, &[FieldValue::Integer(3)]),
            Err(PortError::OutOfRange { .. })
        ));
        assert!(matches!(
            port.delete_row("T", 3),
            Err(PortError::OutOfRange { .. })
        ));
        port.delete_row("T", 0).expect("delete the only row");
        assert!(
            port.read_rows("T", RowRange { start: 0, len: 1 })
                .expect("empty table reads")
                .is_empty()
        );

        // A table without a row entry behaves like an empty table.
        let mut empty = MockPort::with_tables(vec![table()]);
        assert!(matches!(
            empty.update_row("T", 0, &[FieldValue::Integer(1)]),
            Err(PortError::OutOfRange { .. })
        ));
        assert!(matches!(
            empty.delete_row("T", 0),
            Err(PortError::OutOfRange { .. })
        ));
    }

    /// The mock insert validates the table and the value arity itself.
    #[test]
    fn mock_insert_row_checks_table_and_arity() {
        let mut port = MockPort::with_tables(vec![table()]);
        assert!(matches!(
            port.insert_row("missing", &[]),
            Err(PortError::TableNotFound(_))
        ));
        match port.insert_row("t", &[]) {
            Err(PortError::WriteFailed { detail }) => {
                assert!(detail.contains("expected 1 values"), "{detail}");
            }
            other => panic!("expected WriteFailed, got {other:?}"),
        }
        port.insert_row("t", &[FieldValue::Integer(1)])
            .expect("valid insert");
        port.open(std::path::Path::new("memory.mock"), None)
            .expect("open the mock");
        assert_eq!(
            port.read_rows("T", RowRange { start: 0, len: 1 })
                .expect("row is stored"),
            vec![vec![FieldValue::Integer(1)]]
        );
    }
}
