//! Storage error taxonomy (step 2.1, item H4).
//!
//! Errors carry context and detail so malformed input is precisely locatable
//! (LESSONS #4): no silent swallowing anywhere near the parse boundary.

use std::fmt;

/// Errors produced by any storage backend.
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
    /// A page number beyond the end of the file.
    PageOutOfRange {
        /// Requested page number (0-based).
        page: u32,
        /// Number of pages the file actually holds.
        page_count: usize,
    },
    /// The database is password-protected and no password was supplied.
    PasswordRequired,
    /// The supplied database password does not match the stored one.
    WrongPassword,
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
            Self::PageOutOfRange { page, page_count } => {
                write!(
                    f,
                    "page {page} beyond end of file (file has {page_count} pages)"
                )
            }
            Self::PasswordRequired => write!(
                f,
                "the database is password-protected; a password is required"
            ),
            Self::WrongPassword => write!(f, "the database password is wrong"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_arms_name_context_and_detail() {
        let io = StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "db file vanished",
        ));
        assert!(io.to_string().contains("db file vanished"));

        let unsupported = StorageError::Unsupported {
            reason: "RC4-encrypted Jet 3 files".into(),
        };
        assert!(
            unsupported
                .to_string()
                .contains("RC4-encrypted Jet 3 files")
        );

        let out_of_range = StorageError::OutOfRange {
            table: "Customers".into(),
            start: 5,
            len: 2,
            row_count: 3,
        };
        let s = out_of_range.to_string();
        assert!(s.contains("Customers"), "table in display: {s}");
        assert!(s.contains("5..7"), "window in display: {s}");
        assert!(s.contains("3 rows"), "row count in display: {s}");

        let violation = StorageError::SchemaViolation {
            table: "Orders".into(),
            detail: "2 values but 4 columns".into(),
        };
        let s = violation.to_string();
        assert!(s.contains("Orders"), "table in display: {s}");
        assert!(
            s.contains("2 values but 4 columns"),
            "detail in display: {s}"
        );

        let page = StorageError::PageOutOfRange {
            page: 7,
            page_count: 3,
        };
        let s = page.to_string();
        assert!(s.contains("page 7"), "page in display: {s}");
        assert!(s.contains("3 pages"), "page count in display: {s}");
    }

    #[test]
    fn password_errors_are_self_describing() {
        assert!(
            StorageError::PasswordRequired
                .to_string()
                .contains("password is required")
        );
        assert!(
            StorageError::WrongPassword
                .to_string()
                .contains("password is wrong")
        );
    }

    #[test]
    fn io_errors_are_chained_via_source_and_from() {
        let original = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let converted: StorageError = original.into();
        assert!(matches!(converted, StorageError::Io(_)));
        let source = std::error::Error::source(&converted).expect("io variant chains the os error");
        assert!(source.to_string().contains("denied"));

        let corrupt = StorageError::Corrupt {
            context: "x".into(),
            detail: "y".into(),
        };
        assert!(std::error::Error::source(&corrupt).is_none());
    }
}
