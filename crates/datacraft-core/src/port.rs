//! Storage port (step 4.1, item H13).
//!
//! The command core performs storage I/O only through this core-owned
//! trait: real Jet/ACE backends implement it (dependency direction
//! `storage → core`), headless tests use [`MockPort`]. No storage type
//! crosses into the core vocabulary.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::schema::{FieldValue, TableDef};

/// Binary format family of an opened database (domain vocabulary; the
/// backends map their physical version detection onto it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseFormat {
    /// Jet 3.x (Access 97 and earlier).
    Jet3,
    /// Jet 4.x (Access 2000–2003).
    Jet4,
    /// ACE (Access 2007 and later, `.accdb`).
    Ace,
    /// SQLite 3 (`.sqlite`, `.db`) — the non-Jet source family (H35).
    Sqlite,
}

/// Errors a storage backend reports through the port.
#[derive(Debug)]
pub enum PortError {
    /// Opening the database file failed.
    OpenFailed {
        /// Concrete, locatable detail.
        detail: String,
    },
    /// The requested table does not exist.
    TableNotFound(String),
    /// A write operation failed.
    WriteFailed {
        /// Concrete, locatable detail.
        detail: String,
    },
    /// The file is valid but uses a feature the backend does not support.
    Unsupported {
        /// What is unsupported and why.
        reason: String,
    },
    /// A row window starts beyond the table's rows.
    OutOfRange {
        /// Name of the table addressed.
        table: String,
        /// First requested row index.
        start: usize,
        /// Requested window length.
        len: usize,
        /// Actual row count of the table.
        row_count: usize,
    },
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed { detail } => write!(f, "database open failed: {detail}"),
            Self::TableNotFound(name) => write!(f, "table {name:?} not found"),
            Self::WriteFailed { detail } => write!(f, "write failed: {detail}"),
            Self::Unsupported { reason } => write!(f, "unsupported: {reason}"),
            Self::OutOfRange {
                table,
                start,
                len,
                row_count,
            } => write!(
                f,
                "row window {start}+{len} of {table:?} is out of range ({row_count} rows)"
            ),
        }
    }
}

impl std::error::Error for PortError {}

/// One referential-integrity relationship between two tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    /// Table holding the referencing column.
    pub table: String,
    /// Referencing column.
    pub column: String,
    /// Referenced (one-side) table.
    pub related_table: String,
    /// Referenced column.
    pub related_column: String,
}

/// Window selector for row reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowRange {
    /// Zero-based index of the first row to read.
    pub start: usize,
    /// Maximum number of rows to read.
    pub len: usize,
}

/// Metadata of a successfully opened database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenedDatabase {
    /// Database name (file stem).
    pub name: String,
    /// Binary format family.
    pub format: DatabaseFormat,
}

/// The storage seam of the command core. Implementors must be deterministic
/// and must not block indefinitely.
pub trait StoragePort {
    /// Opens the database at `path`.
    fn open(&mut self, path: &std::path::Path) -> Result<OpenedDatabase, PortError>;
    /// Closes the open database (idempotent).
    fn close(&mut self);
    /// Catalog of table definitions in storage order.
    fn tables(&self) -> Vec<TableDef>;
    /// Creates a table; errors on duplicate names.
    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError>;
    /// Drops a table; returns its canonical name.
    fn drop_table(&mut self, name: &str) -> Result<String, PortError>;
    /// Reads a window of rows as domain values. A window starting at
    /// `row_count` is empty; anything beyond fails with
    /// [`PortError::OutOfRange`].
    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError>;
    /// Appends one row (positional values, arity must match the table).
    /// `Null` on an AutoNumber field means "engine-assigned".
    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError>;
    /// Referential relationships of the open database.
    fn relationships(&self) -> Result<Vec<Relationship>, PortError>;
}

/// Window slice shared by implementations: start beyond the count is an
/// error, the tail window is empty.
pub fn window_slice(
    table: &str,
    range: RowRange,
    count: usize,
) -> Result<(usize, usize), PortError> {
    if range.start > count {
        return Err(PortError::OutOfRange {
            table: table.to_owned(),
            start: range.start,
            len: range.len,
            row_count: count,
        });
    }
    let end = range.start.saturating_add(range.len).min(count);
    Ok((range.start, end))
}

/// In-memory backend for headless tests and early UI wiring (mirrors
/// storage's `InMemoryEngine`; no filesystem involvement).
#[derive(Debug, Default)]
pub struct MockPort {
    tables: Vec<TableDef>,
    rows: HashMap<String, Vec<Vec<FieldValue>>>,
    relationships: Vec<Relationship>,
    opened: Option<OpenedDatabase>,
    open_fail: Option<String>,
    write_fail: Option<String>,
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
}

impl StoragePort for MockPort {
    fn open(&mut self, path: &std::path::Path) -> Result<OpenedDatabase, PortError> {
        if let Some(detail) = &self.open_fail {
            return Err(PortError::OpenFailed {
                detail: detail.clone(),
            });
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
        let Some(rows) = self
            .rows
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(table))
            .map(|(_, rows)| rows)
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
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

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(self.relationships.clone())
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
        port.open(std::path::Path::new("memory.mock"))
            .expect("open the mock");
        assert_eq!(
            port.read_rows("T", RowRange { start: 0, len: 1 })
                .expect("row is stored"),
            vec![vec![FieldValue::Integer(1)]]
        );
    }
}
