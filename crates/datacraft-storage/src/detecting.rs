//! Format-detecting storage port (step 13.1, item H35).
//!
//! Clients compose exactly one backend into the core. This port sniffs the
//! file magic and hands the file to the matching backend, so the command
//! vocabulary stays identical for Jet/ACE and SQLite sources.

use std::path::Path;

use datacraft_core::port::{OpenedDatabase, PortError, Relationship, RowRange, StoragePort};
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
    fn open(&mut self, path: &Path) -> Result<OpenedDatabase, PortError> {
        let magic = sniff(path)?;
        let opened = match magic {
            Magic::Jet => {
                let mut port = Jet4Port::new();
                let opened = port.open(path)?;
                self.backend = Some(Backend::Jet(port));
                opened
            }
            Magic::Sqlite => {
                let mut port = SqlitePort::new();
                let opened = port.open(path)?;
                self.backend = Some(Backend::Sqlite(port));
                opened
            }
        };
        Ok(opened)
    }

    fn close(&mut self) {
        self.backend = None;
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
