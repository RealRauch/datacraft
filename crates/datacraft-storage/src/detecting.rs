//! Format-detecting storage port (step 13.1, item H35).
//!
//! Clients compose exactly one backend into the core. This port sniffs the
//! file magic and hands the file to the matching backend, so the command
//! vocabulary stays identical for Jet/ACE and SQLite sources.

use std::path::Path;

use datacraft_core::port::{
    DatabaseFormat, OpenedDatabase, PortError, Relationship, RowRange, StoragePort,
};
use datacraft_core::schema::{FieldValue, TableDef};

use crate::port_adapter::Jet4Port;
use crate::sqlite::SqlitePort;

/// Magic of a Jet/ACE file header (`01 00 00 00` at offset 0).
const JET_MAGIC: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
/// Magic of a SQLite 3 file header.
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Which backend serves the open file.
#[derive(Debug)]
enum Backend {
    Jet(Jet4Port),
    Sqlite(SqlitePort),
}

/// One port over all supported source families, selected by file magic.
#[derive(Debug, Default)]
pub struct DetectingPort {
    backend: Option<Backend>,
}

impl DetectingPort {
    /// Creates an unattached port.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StoragePort for DetectingPort {
    fn open(&mut self, path: &Path, password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        let magic = sniff(path)?;
        let opened = match magic {
            Magic::Jet => {
                let mut port = Jet4Port::new();
                let opened = port.open(path, password)?;
                self.backend = Some(Backend::Jet(port));
                opened
            }
            Magic::Sqlite => {
                let mut port = SqlitePort::new();
                let opened = port.open(path, password)?;
                self.backend = Some(Backend::Sqlite(port));
                opened
            }
        };
        Ok(opened)
    }

    fn close(&mut self) {
        self.backend = None;
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        match format {
            DatabaseFormat::Jet4 => Jet4Port::new().create_database(path, format),
            DatabaseFormat::Sqlite => SqlitePort::new().create_database(path, format),
            other => Err(PortError::Unsupported {
                reason: format!(
                    "creating a {other:?} database file is not supported — create a \
                     Jet4 or SQLite database"
                ),
            }),
        }
    }

    fn tables(&self) -> Vec<TableDef> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.tables(),
            Some(Backend::Sqlite(port)) => port.tables(),
            None => Vec::new(),
        }
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.create_table(definition),
            Some(Backend::Sqlite(port)) => port.create_table(definition),
            None => Err(no_database()),
        }
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.drop_table(name),
            Some(Backend::Sqlite(port)) => port.drop_table(name),
            None => Err(no_database()),
        }
    }

    fn alter_table(
        &mut self,
        table: &str,
        change: datacraft_core::port::SchemaChange,
    ) -> Result<(), PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.alter_table(table, change),
            Some(Backend::Sqlite(port)) => port.alter_table(table, change),
            None => Err(no_database()),
        }
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.insert_row(table, values),
            Some(Backend::Sqlite(port)) => port.insert_row(table, values),
            None => Err(no_database()),
        }
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.update_row(table, row, values),
            Some(Backend::Sqlite(port)) => port.update_row(table, row, values),
            None => Err(no_database()),
        }
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        match &mut self.backend {
            Some(Backend::Jet(port)) => port.delete_row(table, row),
            Some(Backend::Sqlite(port)) => port.delete_row(table, row),
            None => Err(no_database()),
        }
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.read_rows(table, range),
            Some(Backend::Sqlite(port)) => port.read_rows(table, range),
            None => Err(no_database()),
        }
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.relationships(),
            Some(Backend::Sqlite(port)) => port.relationships(),
            None => Ok(Vec::new()),
        }
    }

    fn native_queries(&self) -> Result<Vec<datacraft_core::port::NativeQuery>, PortError> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.native_queries(),
            Some(Backend::Sqlite(port)) => port.native_queries(),
            None => Ok(Vec::new()),
        }
    }

    fn macros(&self) -> Result<Vec<datacraft_core::macro_def::MacroDef>, PortError> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.macros(),
            // SQLite has no macro concept; the core sees an empty inventory.
            Some(Backend::Sqlite(_)) | None => Ok(Vec::new()),
        }
    }

    fn vba_project(&self) -> Result<Option<datacraft_core::vba::VbaProject>, PortError> {
        match &self.backend {
            Some(Backend::Jet(port)) => port.vba_project(),
            // SQLite has no VBA project.
            Some(Backend::Sqlite(_)) | None => Ok(None),
        }
    }
}

/// The two recognized header families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Magic {
    Jet,
    Sqlite,
}

/// Reads the header and classifies it; unknown content fails with a
/// precise, user-renderable reason (LESSONS #4).
fn sniff(path: &Path) -> Result<Magic, PortError> {
    let mut file = std::fs::File::open(path).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", path.display()),
    })?;
    let mut head = [0u8; 16];
    let filled = read_head(&mut file, &mut head).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", path.display()),
    })?;
    if filled >= SQLITE_MAGIC.len() && &head[..SQLITE_MAGIC.len()] == SQLITE_MAGIC {
        return Ok(Magic::Sqlite);
    }
    if filled >= JET_MAGIC.len() && head[..JET_MAGIC.len()] == JET_MAGIC {
        return Ok(Magic::Jet);
    }
    Err(PortError::OpenFailed {
        detail: format!(
            "{}: unrecognized database file — expected a Jet/ACE header (01 00 00 00) \
             or a SQLite header (\"SQLite format 3\")",
            path.display()
        ),
    })
}

/// Fills `head` with up to its length bytes; returns the byte count.
fn read_head(file: &mut std::fs::File, head: &mut [u8; 16]) -> std::io::Result<usize> {
    use std::io::Read;
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}
