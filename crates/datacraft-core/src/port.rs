//! Storage port (step 4.1, item H13).
//!
//! The command core performs storage I/O only through this core-owned
//! trait: real Jet/ACE backends implement it (dependency direction
//! `storage → core`), headless tests use [`MockPort`]. No storage type
//! crosses into the core vocabulary.

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
    /// The database is password-protected and no password was supplied.
    PasswordRequired {
        /// Filesystem path of the database.
        path: String,
    },
    /// The supplied password does not match the stored database password.
    WrongPassword {
        /// Filesystem path of the database.
        path: String,
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
            Self::PasswordRequired { path } => {
                write!(f, "{path}: the database is password-protected")
            }
            Self::WrongPassword { path } => {
                write!(f, "{path}: the database password is wrong")
            }
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

/// Query flavor of a native saved query (step 15.3, M3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeQueryKind {
    /// Select query (object flags 0x00).
    Select,
    /// Crosstab query (0x10).
    Crosstab,
    /// Delete query (0x20).
    Delete,
    /// Update query (0x30).
    Update,
    /// Append query (0x40).
    Append,
    /// Make-table query (0x50).
    MakeTable,
    /// Data-definition query (0x60).
    DataDefinition,
    /// Pass-through query (0x70).
    Passthrough,
    /// Union query (0x80).
    Union,
}

/// One column of a native saved query (M3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeQueryColumn {
    /// Source table, when the column names one.
    pub table: Option<String>,
    /// Source field or output alias.
    pub field: Option<String>,
    /// Column expression, when the column is computed.
    pub expression: Option<String>,
    /// Whether the column sorts descending.
    pub descending: bool,
}

/// One native saved query as stored in the Jet/ACE file (M3, read-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeQuery {
    /// Query name from MSysObjects.
    pub name: String,
    /// Query flavor from the object flags.
    pub kind: NativeQueryKind,
    /// FROM tables in attribute order.
    pub tables: Vec<String>,
    /// Selected columns.
    pub columns: Vec<NativeQueryColumn>,
    /// WHERE/GROUP BY/HAVING expressions verbatim.
    pub criteria: Vec<String>,
    /// JOIN expressions verbatim.
    pub joins: Vec<String>,
    /// ORDER BY fields.
    pub order_by: Vec<String>,
}

impl NativeQuery {
    /// Maps the decodable subset onto a QBE query definition: select
    /// queries over exactly one table, plain field columns, no criteria,
    /// grouping, joins or computed columns. Everything else stays a
    /// documented rest and answers `None`.
    pub fn query_def(&self) -> Option<crate::qbe::QueryDef> {
        if self.kind != NativeQueryKind::Select
            || self.tables.len() != 1
            || !self.criteria.is_empty()
            || !self.joins.is_empty()
        {
            return None;
        }
        let mut fields = Vec::with_capacity(self.columns.len());
        for column in &self.columns {
            if column.expression.is_some() {
                return None;
            }
            let field = column.field.as_ref()?;
            fields.push(crate::qbe::QbeField {
                name: field.clone(),
                shown: true,
                sort: if column.descending {
                    crate::qbe::SortOrder::Descending
                } else {
                    crate::qbe::SortOrder::Ascending
                },
                criteria: Vec::new(),
            });
        }
        Some(crate::qbe::QueryDef {
            name: self.name.clone(),
            table: self.tables[0].clone(),
            fields,
        })
    }
}

/// The storage seam of the command core. Implementors must be deterministic
/// and must not block indefinitely.
pub trait StoragePort {
    /// Opens the database at `path`; `password` is `None` for unprotected
    /// files (step 16.2, M5). Backends without passwords ignore it.
    fn open(
        &mut self,
        path: &std::path::Path,
        password: Option<&str>,
    ) -> Result<OpenedDatabase, PortError>;
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
    /// Replaces the row at `row` (zero-based live-row index) with `values`
    /// (step 11.10, H40). AutoNumber values are preserved by the caller.
    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError>;
    /// Deletes the row at `row` (zero-based live-row index).
    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError>;
    /// Applies a batch of row operations in order (step 11.11, M2). Indices
    /// address the row set as it evolves through the batch. Backends apply
    /// the batch atomically where they can (Jet does); the default applies
    /// the operations one by one.
    fn write_row_ops(&mut self, table: &str, ops: &[RowWriteOp]) -> Result<(), PortError> {
        for op in ops {
            match op {
                RowWriteOp::Insert(values) => self.insert_row(table, values)?,
                RowWriteOp::Update { row, values } => self.update_row(table, *row, values)?,
                RowWriteOp::Delete { row } => self.delete_row(table, *row)?,
            }
        }
        Ok(())
    }
    /// Referential relationships of the open database.
    fn relationships(&self) -> Result<Vec<Relationship>, PortError>;
    /// Native saved queries of the open database (step 15.3, M3).
    /// Backends without native queries return an empty list.
    fn native_queries(&self) -> Result<Vec<NativeQuery>, PortError> {
        Ok(Vec::new())
    }
    /// Named macros of the open database (step 17.1, L4). Backends without
    /// macros return an empty list; definitions are read-only.
    fn macros(&self) -> Result<Vec<crate::macro_def::MacroDef>, PortError> {
        Ok(Vec::new())
    }
}

/// One row operation of a batch write (the journal's port vocabulary).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RowWriteOp {
    /// Appends a row.
    Insert(Vec<FieldValue>),
    /// Replaces the row at `row`.
    Update {
        /// Zero-based live-row index in the evolving row set.
        row: usize,
        /// Full positional value list.
        values: Vec<FieldValue>,
    },
    /// Removes the row at `row`.
    Delete {
        /// Zero-based live-row index in the evolving row set.
        row: usize,
    },
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

pub use crate::port_mock::MockPort;
