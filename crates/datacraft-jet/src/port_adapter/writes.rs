//! The writer plumbing of the Jet4 port (the proactive L30 split of
//! `port_adapter.rs`): table identity lookups and the single writer
//! choke point every write path funnels through — including the Access
//! lock-file guard (step 53.1, item M59).

use datacraft_core::port::PortError;
use datacraft_core::schema::TableDef;

use crate::jet::complex::ComplexCatalog;
use crate::jet::writer::Jet4Writer;
use crate::jet::reader::Jet4TableReader;
use crate::error::StorageError;
use crate::port_errors::{map_open, map_write, no_database};

use super::Jet4Port;

impl Jet4Port {
    /// TDEF page of a catalogued table (the writer's identity).
    pub(super) fn page_of(&self, table: &str) -> Result<u32, PortError> {
        self.pages
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(table))
            .map(|(_, page)| *page)
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Domain definition of a catalogued table.
    pub(super) fn definition_of(&self, table: &str) -> Result<&TableDef, PortError> {
        self.tables
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))
    }

    /// Runs one writer operation on a private copy of the file, persists the
    /// result and swaps in the refreshed reader/catalog. A failed operation
    /// leaves the file and the in-memory state untouched. A locked session
    /// (a sibling Access lock file) answers the precise read-only refusal
    /// before any writer work starts (step 53.1, item M59).
    pub(super) fn with_writer<T>(
        &mut self,
        op: impl FnOnce(&mut Jet4Writer) -> Result<T, StorageError>,
    ) -> Result<T, PortError> {
        if self.lock_file.is_some() {
            return Err(self.locked_error());
        }
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
        datacraft_core::atomic_write::write_replacing(&path, &disk).map_err(|e| {
            PortError::WriteFailed {
                detail: format!("{}: {e}", path.display()),
            }
        })?;
        let complex = ComplexCatalog::read(&reader).map_err(map_open)?;
        let entries = super::collect_tables(&reader, &complex)?;
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
