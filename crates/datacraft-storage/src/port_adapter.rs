//! `StoragePort` adapter (item H16): the Jet4 read backend behind the
//! core-owned port trait (dependency direction `storage → core`).
//!
//! Mapping decisions (documented in `docs/FORMAT-JET.md` §"Domain mapping"):
//!
//! - System tables are excluded by the `MSys` name prefix — the observed
//!   writer-stable convention (their `Flags` values vary per writer:
//!   `0x80000000`, `0x2`, `0xa`).
//! - Plain GUID columns (Jet `0x0F`) map to `FieldKind::Guid` (H12
//!   decision); GUID AutoNumbers map to `AutoNumber(ReplicationId)`.
//! - Text length comes from the descriptor: uncompressed (UTF-16) columns
//!   carry byte length, so the character budget is `len / 2`; compressed
//!   columns are byte-per-char.
//! - Jet `Numeric` descriptors carry only the scale; precision is modeled
//!   as the 96-bit maximum (28).
//! - Writes run through the clean-room Jet4 writer behind the port (step
//!   11.1); shapes the writer does not qualify yet (AutoNumber counters,
//!   attachments, table drops) answer a precise `Unsupported`
//!   (BACKLOG M4/M7/M8).

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, RowRange, StoragePort,
    Support,
};
use datacraft_core::schema::{FieldValue, TableDef};

use crate::error::StorageError;
use crate::jet::OBJECT_TYPE_TABLE;
use crate::jet::complex::ComplexCatalog;
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::JetTableDef;
use crate::jet::writer::Jet4Writer;
use crate::port_errors::{map_open, map_open_with_path, map_write, no_database};
use crate::port_schema::table_def;
use crate::port_write;
use crate::types::FormatVersion;

/// The Jet4 backend as a [`StoragePort`] (read/write).
#[derive(Debug, Default)]
pub struct Jet4Port {
    reader: Option<Jet4TableReader>,
    tables: Vec<TableDef>,
    /// TDEF page of every catalogued table (the writer's identity).
    pages: Vec<(String, u32)>,
    /// Complex-column mapping (`MSysComplexColumns`); empty for Jet4.
    complex: ComplexCatalog,
    /// Named macros read at open (step 17.1, L4; read-only).
    macros: Vec<datacraft_core::macro_def::MacroDef>,
    /// Extracted VBA project (step 17.2, L5; read-only).
    vba: Option<datacraft_core::vba::VbaProject>,
    /// Why the VBA extraction failed, if it did (observable degradation).
    vba_error: Option<String>,
    path: Option<PathBuf>,
}

impl Jet4Port {
    /// Creates an unattached adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// The physical format of the open database, if any.
    fn opened_format(&self) -> Option<FormatVersion> {
        self.reader.as_ref().map(|reader| reader.header().format)
    }
}

impl StoragePort for Jet4Port {
    fn driver_id(&self) -> DriverId {
        DriverId::new("jet")
    }

    /// Per-session capabilities (phase 30, M17; ADR-7): Jet4/ACE are the
    /// read/write reference; an open Jet3 (Access 97) session is
    /// read-only — no writer exists for it. File creation is
    /// session-independent (Jet4 only, routed in `create_database`).
    fn capabilities(&self) -> DriverCapabilities {
        let mut caps = DriverCapabilities::full();
        caps.atomic_batch = true; // batches apply in one writer session
        caps.passwords = true;
        caps.autonumber = Support::Yes; // LongInteger; ReplicationId answers Unsupported
        if self.opened_format() == Some(FormatVersion::Jet3) {
            caps.write_rows = false;
            caps.create_table = false;
            caps.drop_table = false;
            caps.alter_table = false;
            caps.atomic_batch = false;
            caps.autonumber = Support::No;
        }
        caps
    }

    fn open(&mut self, path: &Path, password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        let bytes = std::fs::read(path).map_err(|e| PortError::OpenFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let reader = Jet4TableReader::new_with_password(bytes, password)
            .map_err(|error| map_open_with_path(error, path))?;
        let format = match reader.header().format {
            FormatVersion::Jet3 => DatabaseFormat::Jet3,
            FormatVersion::Jet4 => DatabaseFormat::Jet4,
            FormatVersion::Ace => DatabaseFormat::Ace,
        };
        let complex = ComplexCatalog::read(&reader).map_err(map_open)?;
        let entries = collect_tables(&reader, &complex)?;
        let macros = crate::port_objects::macros_to_core(reader.macros().map_err(map_open)?);
        let (vba, vba_error) = match reader.vba_project() {
            Ok(project) => (project.map(crate::port_objects::vba_to_core), None),
            // Extraction failure is reported through the port, not by
            // blocking the database open.
            Err(error) => (None, Some(error.to_string())),
        };
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.complex = complex;
        self.macros = macros;
        self.vba = vba;
        self.vba_error = vba_error;
        self.reader = Some(reader);
        self.path = Some(path.to_path_buf());
        Ok(OpenedDatabase { name, format })
    }

    fn close(&mut self) {
        self.reader = None;
        self.tables.clear();
        self.pages.clear();
        self.complex = ComplexCatalog::default();
        self.macros.clear();
        self.vba = None;
        self.vba_error = None;
        self.path = None;
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        match format {
            DatabaseFormat::Jet4 => {
                let created = crate::jet::encode::now_as_ole_date();
                crate::jet::create::create_jet4_file(path, created).map_err(|error| {
                    PortError::WriteFailed {
                        detail: format!("{}: {error}", path.display()),
                    }
                })
            }
            DatabaseFormat::Jet3 => Err(PortError::Unsupported {
                reason: "Jet3 (Access 97) files are never created — create a Jet4 database"
                    .to_owned(),
            }),
            DatabaseFormat::Ace => Err(PortError::Unsupported {
                reason: "ACE 2007+ files cannot be created yet — create a Jet4 database".to_owned(),
            }),
            DatabaseFormat::Sqlite => Err(PortError::Unsupported {
                reason: "SQLite files are created by the SQLite backend, not the Jet backend"
                    .to_owned(),
            }),
            DatabaseFormat::Csv => Err(PortError::Unsupported {
                reason: "CSV directories are created by the CSV driver, not the Jet backend"
                    .to_owned(),
            }),
        }
    }

    /// Writes a copy of the open database (step 27.2, item M14). The copy
    /// is derived from the decrypted writer handoff: the header region is
    /// rewritten with the requested password state and page-encrypted
    /// accordingly. The source file is never touched.
    fn save_as(&mut self, path: &Path, password: Option<&str>) -> Result<(), PortError> {
        let reader = self.reader.as_ref().ok_or_else(no_database)?;
        if reader.header.format == FormatVersion::Jet3 {
            return Err(PortError::Unsupported {
                reason: "Jet3 (Access 97) sources are read-only — save-as needs a writable \
                         format"
                    .to_owned(),
            });
        }
        let header = reader.header.clone();
        let mut copy = reader.file.clone();
        let encryption =
            crate::jet::crypto::set_password(&mut copy, &header, password).map_err(map_write)?;
        crate::jet::crypto::encrypt_file(&mut copy, &header, encryption.as_ref())
            .map_err(map_write)?;
        write_new_file(path, &copy)
    }

    /// Rewrites the open file with a new password (step 27.2, item M14).
    /// The reader swaps to the newly encrypted state, so subsequent writes
    /// re-encrypt with the new key.
    fn change_password(&mut self, new_password: Option<&str>) -> Result<(), PortError> {
        let path = self.path.clone().ok_or_else(no_database)?;
        let (header, decrypted) = {
            let reader = self.reader.as_ref().ok_or_else(no_database)?;
            (reader.header.clone(), reader.file.clone())
        };
        let mut decrypted = decrypted;
        let encryption = crate::jet::crypto::set_password(&mut decrypted, &header, new_password)
            .map_err(map_write)?;
        let mut disk = decrypted.clone();
        crate::jet::crypto::encrypt_file(&mut disk, &header, encryption.as_ref())
            .map_err(map_write)?;
        write_replacing(&path, &disk)?;
        let reader = Jet4TableReader::from_decrypted(decrypted, encryption).map_err(map_open)?;
        self.reader = Some(reader);
        Ok(())
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        let columns = port_write::table_to_columns(definition)?;
        let name = definition.name.clone();
        self.with_writer(|writer| writer.create_table(&name, &columns))?;
        Ok(())
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let Some((canonical, page)) = self
            .pages
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(candidate, page)| (candidate.clone(), *page))
        else {
            return Err(PortError::TableNotFound(name.to_owned()));
        };
        self.with_writer(|writer| writer.drop_table(page))?;
        Ok(canonical)
    }

    fn alter_table(
        &mut self,
        table: &str,
        change: datacraft_core::port::SchemaChange,
    ) -> Result<(), PortError> {
        if self.reader.is_none() {
            return Err(no_database());
        }
        let Some(page) = self
            .pages
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(table))
            .map(|(_, page)| *page)
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
        self.with_writer(|writer| writer.alter_table(page, &change))?;
        Ok(())
    }

    fn native_queries(&self) -> Result<Vec<datacraft_core::port::NativeQuery>, PortError> {
        let Some(reader) = self.reader.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(crate::port_native::to_core(
            reader.native_queries().map_err(map_open)?,
        ))
    }

    fn macros(&self) -> Result<Vec<datacraft_core::macro_def::MacroDef>, PortError> {
        Ok(self.macros.clone())
    }

    fn vba_project(&self) -> Result<Option<datacraft_core::vba::VbaProject>, PortError> {
        if let Some(reason) = &self.vba_error {
            return Err(PortError::Unsupported {
                reason: reason.clone(),
            });
        }
        Ok(self.vba.clone())
    }

    fn relationships(&self) -> Result<Vec<datacraft_core::port::Relationship>, PortError> {
        let reader = self.reader.as_ref().ok_or_else(|| PortError::Unsupported {
            reason: "no database is open".to_owned(),
        })?;
        let data = match reader.open_table("MSysRelationships") {
            Ok(data) => data,
            // No relationships table: the database has none.
            Err(crate::error::StorageError::TableNotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(map_open(e)),
        };
        let index_of = |name: &str| {
            data.schema
                .columns
                .iter()
                .position(|c| c.name == name)
                .ok_or_else(|| PortError::Unsupported {
                    reason: format!("MSysRelationships lacks the {name} column"),
                })
        };
        let (i_obj, i_col, i_ref_obj, i_ref_col) = (
            index_of("szObject")?,
            index_of("szColumn")?,
            index_of("szReferencedObject")?,
            index_of("szReferencedColumn")?,
        );
        let text = |v: &crate::types::StorageValue| match v {
            crate::types::StorageValue::Text(s) => Some(s.clone()),
            _ => None,
        };
        Ok(data
            .rows
            .iter()
            .filter_map(|row| {
                Some(datacraft_core::port::Relationship {
                    table: text(&row[i_obj])?,
                    column: text(&row[i_col])?,
                    related_table: text(&row[i_ref_obj])?,
                    related_column: text(&row[i_ref_col])?,
                })
            })
            .collect())
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        rows::insert_row(self, table, values)
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        rows::read_rows(self, table, range)
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        rows::update_row(self, table, row, values)
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        rows::delete_row(self, table, row)
    }

    fn write_row_ops(
        &mut self,
        table: &str,
        ops: &[datacraft_core::port::RowWriteOp],
    ) -> Result<(), PortError> {
        rows::write_row_ops(self, table, ops)
    }
}

mod rows;

impl Jet4Port {
    /// TDEF page of a catalogued table (the writer's identity).
    fn page_of(&self, table: &str) -> Result<u32, PortError> {
        self.pages
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(table))
            .map(|(_, page)| *page)
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Domain definition of a catalogued table.
    fn definition_of(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Runs one writer operation on a private copy of the file, persists the
    /// result and swaps in the refreshed reader/catalog. A failed operation
    /// leaves the file and the in-memory state untouched.
    fn with_writer<T>(
        &mut self,
        op: impl FnOnce(&mut Jet4Writer) -> Result<T, StorageError>,
    ) -> Result<T, PortError> {
        let path = self.path.clone().ok_or_else(no_database)?;
        let (bytes, encryption) = {
            let reader = self.reader.as_ref().ok_or_else(no_database)?;
            (reader.file.clone(), reader.encryption().copied())
        };
        let mut writer = Jet4Writer::new(bytes).map_err(map_write)?;
        let out = op(&mut writer).map_err(map_write)?;
        let decrypted = writer.into_file();
        // The writer works on decrypted bytes; the on-disk file keeps the
        // header region encrypted and, for protected databases, every page
        // after page 0 (step 16.2, M5).
        let reader = Jet4TableReader::from_decrypted(decrypted, encryption).map_err(map_open)?;
        let header = reader.header().clone();
        let mut disk = reader.file.clone();
        crate::jet::crypto::encrypt_file(&mut disk, &header, encryption.as_ref())
            .map_err(map_open)?;
        std::fs::write(&path, &disk).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        })?;
        let complex = ComplexCatalog::read(&reader).map_err(map_open)?;
        let entries = collect_tables(&reader, &complex)?;
        self.tables = entries.iter().map(|(def, _)| def.clone()).collect();
        self.pages = entries
            .into_iter()
            .map(|(def, page)| (def.name, page))
            .collect();
        self.complex = complex;
        self.reader = Some(reader);
        Ok(out)
    }
}

/// Writes a new file, refusing an existing target; a failed write removes
/// the partial file so no half-copy survives (step 27.2, M14).
fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), PortError> {
    use std::io::Write;
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(PortError::WriteFailed {
                detail: format!("{}: the target file already exists", path.display()),
            });
        }
        Err(e) => {
            return Err(PortError::WriteFailed {
                detail: format!("{}: {e}", path.display()),
            });
        }
    };
    if let Err(e) = file.write_all(bytes) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        });
    }
    Ok(())
}

/// Atomically replaces `path` through a sibling temp file (the settings
/// pattern from step 22.1): an interrupted write can never truncate the
/// existing database.
fn write_replacing(path: &Path, bytes: &[u8]) -> Result<(), PortError> {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    let temp = PathBuf::from(name);
    if let Err(e) = std::fs::write(&temp, bytes) {
        let _ = std::fs::remove_file(&temp);
        return Err(PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        });
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        });
    }
    Ok(())
}

/// Enumerates user tables through the catalog with their TDEF pages.
/// Complex backing tables (`f_<guid>_*`) are hidden — they belong to their
/// owning column, not to the navigation pane.
fn collect_tables(
    reader: &Jet4TableReader,
    complex: &ComplexCatalog,
) -> Result<Vec<(TableDef, u32)>, PortError> {
    let mut tables = Vec::new();
    for entry in reader.catalog().map_err(map_open)? {
        if entry.object_type != OBJECT_TYPE_TABLE {
            continue;
        }
        let Some(tdef_page) = entry.tdef_page else {
            continue;
        };
        if entry.name.starts_with("MSys") || complex.is_flat_page(tdef_page) {
            continue;
        }
        let def = JetTableDef::decode(&reader.file, tdef_page).map_err(map_open)?;
        tables.push((
            table_def(&entry.name, tdef_page, &def, reader, complex)?,
            tdef_page,
        ));
    }
    Ok(tables)
}

#[cfg(test)]
#[path = "port_adapter_tests.rs"]
mod tests;
