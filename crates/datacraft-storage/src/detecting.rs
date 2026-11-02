//! Format-detecting storage port — the driver registry facade (ADR-7).
//!
//! Clients compose exactly one port into the core. On open, the header
//! probe builds the driver that claims the file; every trait call then
//! delegates to that open session, so the session handling is
//! driver-agnostic (adding a driver is one probe entry, nothing else).
//! While nothing is open, the composite speaks for itself.

use std::path::Path;

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, SchemaChange, StoragePort,
};
use datacraft_core::schema::{FieldValue, TableDef};

use crate::port_adapter::Jet4Port;
use crate::sqlite::SqlitePort;

/// One port over all supported source families, selected by file magic.
#[derive(Default)]
pub struct DetectingPort {
    /// The open session, whatever driver claimed the file.
    session: Option<Box<dyn StoragePort>>,
    /// Out-of-tree driver factory (ADR-7): consulted when no built-in
    /// probe claims the path, and for `create_database` formats the
    /// built-ins don't serve. The driver answers for itself or fails
    /// the open precisely.
    fallback: Option<Box<dyn Fn() -> Box<dyn StoragePort> + Send + Sync>>,
}

impl std::fmt::Debug for DetectingPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetectingPort")
            .field(
                "session",
                &self.session.as_ref().map(|session| session.driver_id()),
            )
            .field("fallback", &self.fallback.is_some())
            .finish()
    }
}

impl DetectingPort {
    /// Creates an unattached port.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an out-of-tree driver factory (ADR-7): the composition
    /// roots compose the engine with their driver set this way — the
    /// storage crate never names the extra driver crates.
    pub fn with_fallback(
        mut self,
        factory: impl Fn() -> Box<dyn StoragePort> + Send + Sync + 'static,
    ) -> Self {
        self.fallback = Some(Box::new(factory));
        self
    }
}

impl StoragePort for DetectingPort {
    fn driver_id(&self) -> DriverId {
        self.session
            .as_ref()
            .map_or_else(|| DriverId::new("detecting"), |session| session.driver_id())
    }

    /// The open session's capabilities (ADR-7); while nothing is open the
    /// optimistic default holds — creation is session-independent.
    fn capabilities(&self) -> DriverCapabilities {
        self.session
            .as_ref()
            .map_or_else(DriverCapabilities::full, |session| session.capabilities())
    }

    fn open(&mut self, path: &Path, password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        let mut driver = match probe(path) {
            Ok(driver) => driver,
            Err(error) => {
                let Some(factory) = &self.fallback else {
                    return Err(error);
                };
                let mut fallback = factory();
                let opened = fallback.open(path, password)?;
                self.session = Some(fallback);
                return Ok(opened);
            }
        };
        let opened = driver.open(path, password)?;
        self.session = Some(driver);
        Ok(opened)
    }

    fn close(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.close();
        }
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        match format {
            DatabaseFormat::Jet4 => Jet4Port::new().create_database(path, format),
            DatabaseFormat::Sqlite => SqlitePort::new().create_database(path, format),
            other => match &self.fallback {
                Some(factory) => factory().create_database(path, other),
                None => Err(PortError::Unsupported {
                    reason: format!(
                        "creating a {other:?} database file is not supported — create a \
                         Jet4 or SQLite database"
                    ),
                }),
            },
        }
    }

    fn save_as(&mut self, path: &Path, password: Option<&str>) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .save_as(path, password)
    }

    fn change_password(&mut self, new_password: Option<&str>) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .change_password(new_password)
    }

    fn tables(&self) -> Vec<TableDef> {
        self.session
            .as_ref()
            .map_or_else(Vec::new, |session| session.tables())
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .create_table(definition)
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .drop_table(name)
    }

    fn alter_table(&mut self, table: &str, change: SchemaChange) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .alter_table(table, change)
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .insert_row(table, values)
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .update_row(table, row, values)
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        self.session
            .as_mut()
            .ok_or_else(no_database)?
            .delete_row(table, row)
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        self.session
            .as_ref()
            .ok_or_else(no_database)?
            .read_rows(table, range)
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        self.session
            .as_ref()
            .map_or_else(|| Ok(Vec::new()), |session| session.relationships())
    }

    fn native_queries(&self) -> Result<Vec<datacraft_core::port::NativeQuery>, PortError> {
        self.session
            .as_ref()
            .map_or_else(|| Ok(Vec::new()), |session| session.native_queries())
    }

    fn macros(&self) -> Result<Vec<datacraft_core::macro_def::MacroDef>, PortError> {
        self.session
            .as_ref()
            .map_or_else(|| Ok(Vec::new()), |session| session.macros())
    }

    fn vba_project(&self) -> Result<Option<datacraft_core::vba::VbaProject>, PortError> {
        self.session
            .as_ref()
            .map_or_else(|| Ok(None), |session| session.vba_project())
    }
}

/// The recognized header families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Magic {
    Jet,
    Sqlite,
}

/// Probes the file header and builds the driver that claims it (ADR-7:
/// one entry per driver — the session handling above never changes).
fn probe(path: &Path) -> Result<Box<dyn StoragePort>, PortError> {
    match sniff(path)? {
        Magic::Jet => Ok(Box::new(Jet4Port::new())),
        Magic::Sqlite => Ok(Box::new(SqlitePort::new())),
    }
}

/// Reads the header and classifies it; unknown content fails with a
/// precise, user-renderable reason (LESSONS #4).
fn sniff(path: &Path) -> Result<Magic, PortError> {
    const JET_MAGIC: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
    const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";
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
