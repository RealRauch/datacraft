//! Storage port (step 4.1, item H13).
//!
//! The command core performs storage I/O only through this core-owned
//! trait: real Jet/ACE backends implement it (dependency direction
//! `storage → core`), headless tests use [`MockPort`]. No storage type
//! crosses into the core vocabulary.

use serde::{Deserialize, Serialize};

use crate::schema::TableDef;

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
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed { detail } => write!(f, "database open failed: {detail}"),
            Self::TableNotFound(name) => write!(f, "table {name:?} not found"),
            Self::WriteFailed { detail } => write!(f, "write failed: {detail}"),
            Self::Unsupported { reason } => write!(f, "unsupported: {reason}"),
        }
    }
}

impl std::error::Error for PortError {}

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
}

/// In-memory backend for headless tests and early UI wiring (mirrors
/// storage's `InMemoryEngine`; no filesystem involvement).
#[derive(Debug, Default)]
pub struct MockPort {
    tables: Vec<TableDef>,
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
}
