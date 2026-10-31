//! Storage error taxonomy (step 2.1, item H4).
//!
//! Errors carry context and detail so malformed input is precisely locatable
//! (LESSONS #4): no silent swallowing anywhere near the parse boundary.

use std::fmt;

/// Errors produced by any [`StorageEngine`](crate::StorageEngine) backend.
#[derive(Debug)]
pub enum StorageError {
    /// Underlying file I/O failure.
    Io(std::io::Error),
    /// The file structure is damaged or does not match the expected format.
    Corrupt {
        /// Where in the file/format the problem was detected (e.g. `"page header"`).
        context: String,
        /// Concrete, locatable detail (offsets, expected vs. actual bytes).
        detail: String,
    },
    /// The file is valid but uses a feature this backend does not support.
    Unsupported {
        /// What is unsupported and why.
        reason: String,
    },
    /// The requested table does not exist.
    TableNotFound(String),
    /// The requested row window lies beyond the table's rows.
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
    /// A write operation violated the table schema (e.g. wrong value count).
    SchemaViolation {
        /// Name of the table addressed.
        table: String,
        /// Concrete violation detail.
        detail: String,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "storage I/O error: {e}"),
            Self::Corrupt { context, detail } => {
                write!(f, "corrupt database at {context}: {detail}")
            }
            Self::Unsupported { reason } => write!(f, "unsupported feature: {reason}"),
            Self::TableNotFound(t) => write!(f, "table not found: {t}"),
            Self::OutOfRange {
                table,
                start,
                len,
                row_count,
            } => write!(
                f,
                "row window out of range on table {table}: requested rows \
                 {start}..{} but table has {row_count} rows",
                start + len
            ),
            Self::SchemaViolation { table, detail } => {
                write!(f, "schema violation on table {table}: {detail}")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
