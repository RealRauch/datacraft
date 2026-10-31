//! Backend-agnostic storage domain types (step 2.1, item H4).
//!
//! These types form the vocabulary of the [`StorageEngine`](crate::StorageEngine)
//! boundary (ARCHITECTURE §6): physical Jet/ACE concerns are mapped onto them
//! and no format-specific type may leak across the boundary. Logical Access
//! semantics (e.g. "Short Text", "AutoNumber") are a Phase 3 concern and map
//! onto these physical types.

use serde::{Deserialize, Serialize};

/// Binary format family and version of a database file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FormatVersion {
    /// Jet 3.x (Access 97 and earlier).
    Jet3,
    /// Jet 4.x (Access 2000–2003).
    Jet4,
    /// ACE (Access 2007 and later, `.accdb`).
    Ace,
}

/// Physical storage type of a column as encoded in the database file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageDataType {
    /// Boolean, stored as a 1-byte bitfield.
    Bit,
    /// Unsigned/signed 1-byte integer.
    Int1,
    /// 2-byte integer.
    Int2,
    /// 4-byte integer.
    Int4,
    /// IEEE 754 single precision.
    Float4,
    /// IEEE 754 double precision.
    Float8,
    /// Scaled 64-bit integer (factor 10⁻⁴).
    Currency,
    /// OLE Automation DATE (f64, days since 1899-12-30).
    DateTime,
    /// Short text (fixed or variable length, in-row).
    Text,
    /// Long text stored on memo pages.
    Memo,
    /// Binary data (fixed/variable, in-row or multi-page).
    Binary,
    /// 16-byte replication identifier.
    Guid,
    /// 96-bit scaled decimal (17-byte storage in the file).
    Decimal,
    /// ACE multi-value/attachment columns.
    Complex,
}

/// A single stored value; `Null` is a distinct value, not a sentinel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageValue {
    /// SQL NULL.
    Null,
    /// Boolean value.
    Bit(bool),
    /// Integral value (widened from 1/2/4-byte storage).
    Int(i64),
    /// Floating-point value.
    Float(f64),
    /// Currency value in scaled integer form (factor 10⁻⁴).
    Currency(i64),
    /// Date/time in OLE Automation form.
    DateTime(f64),
    /// Text value.
    Text(String),
    /// Raw binary value.
    Binary(Vec<u8>),
    /// 16-byte GUID.
    Guid([u8; 16]),
    /// Scaled decimal value (scale carried by the column schema).
    Decimal(i128),
}

/// Definition of one column (name, physical type, nullability).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnSchema {
    /// Column name as stored in the database.
    pub name: String,
    /// Physical storage type.
    pub data_type: StorageDataType,
    /// Whether the column may hold `Null`.
    pub nullable: bool,
}

impl ColumnSchema {
    /// Creates a column definition.
    pub fn new(name: impl Into<String>, data_type: StorageDataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }
}

/// Schema of one table: name plus ordered column definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSchema {
    /// Table name as stored in the database.
    pub name: String,
    /// Column definitions in storage order.
    pub columns: Vec<ColumnSchema>,
}

impl TableSchema {
    /// Creates a table schema.
    pub fn new(name: impl Into<String>, columns: Vec<ColumnSchema>) -> Self {
        Self {
            name: name.into(),
            columns,
        }
    }

    /// Number of columns (arity of every row of this table).
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

/// Window selector for [`StorageEngine::read_rows`](crate::StorageEngine::read_rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowRange {
    /// Zero-based index of the first row to read.
    pub start: usize,
    /// Maximum number of rows to read.
    pub len: usize,
}

impl RowRange {
    /// Creates a window starting at `start` with at most `len` rows.
    pub fn at(start: usize, len: usize) -> Self {
        Self { start, len }
    }
}

/// A row of values, aligned positionally with the table's columns.
pub type Row = Vec<StorageValue>;

/// Result window returned by
/// [`StorageEngine::read_rows`](crate::StorageEngine::read_rows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowBatch {
    /// Zero-based index of the first returned row.
    pub start_row: usize,
    /// Returned rows, positional to the table schema.
    pub rows: Vec<Row>,
}

/// One write operation. Indices address the row set as it **evolves through
/// the batch** (sequential application).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RowOp {
    /// Appends a row (positional values, `Null` allowed per schema).
    Insert(Vec<StorageValue>),
    /// Replaces all values of the row at the given index.
    Update {
        /// Index of the row to replace (current batch state).
        row: usize,
        /// Full positional value list (arity must match the schema).
        values: Vec<StorageValue>,
    },
    /// Removes the row at the given index.
    Delete {
        /// Index of the row to remove (current batch state).
        row: usize,
    },
}

/// Summary of a successful
/// [`StorageEngine::write_rows`](crate::StorageEngine::write_rows) batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteReport {
    /// Number of rows appended.
    pub inserted: usize,
    /// Number of rows replaced.
    pub updated: usize,
    /// Number of rows removed.
    pub deleted: usize,
}

/// Metadata returned when opening a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseMeta {
    /// Binary format family/version of the opened file.
    pub format: FormatVersion,
}

impl DatabaseMeta {
    /// Creates metadata for the given format.
    pub fn new(format: FormatVersion) -> Self {
        Self { format }
    }
}
