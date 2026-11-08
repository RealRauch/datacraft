//! Fresh Jet4 file builder and template materializer (phase 29, steps
//! 29.1/29.2, item H44; engine-shaped fresh catalog step 49.2, item M63).
//!
//! A brand-new database is the embedded minimal engine-written template
//! (`FRESH_TEMPLATE`, 19 pages), carried verbatim except the page-0
//! tail's first word (normalized to the uniform `0x0100` writer policy).
//! The template is DataCraft's own black-box product
//! (`tools/dao-oracle/oracle.ps1 new-db`; clean-room rule untouched,
//! ARCHITECTURE §12), so the engine-written shape — catalog, system
//! tables, usage maps, index metadata and pages — passes the engine's
//! open gates by construction (`docs/FORMAT-JET.md` "Fresh-create
//! template").
//!
//! The template's creation date is **not** redated: the engine binds its
//! ACL validation to the creation time-of-day the template was written
//! with (oracle mutation matrix, 49.2 — a redated header answers the
//! engine's permission-denied refusal). Every file therefore carries the
//! fixed template date; `created` stamps only the rows the writer adds
//! afterwards (template materialization), which the engine does not gate.
//!
//! Determinism (LESSONS #6): the fresh bytes are a constant; the only
//! input to a materialized file is the template's table set and the
//! explicit `created` date — same inputs produce byte-identical files,
//! which is what the sample generator (step 29.3) pins with `--check`.

use std::path::Path;

use datacraft_core::schema::{FieldValue, TableDef};

use crate::error::StorageError;
use crate::jet::writer::Jet4Writer;
use crate::port_write;

/// Page-0 offset of the 256-word tail's first word.
const TAIL_AT: usize = 0xE00;

/// The embedded minimal engine-written template (provenance:
/// `tools/dao-oracle/oracle.ps1 new-db`, 2026-11-06; the create suite pins
/// the emitted bytes, so any template drift shifts the deterministic
/// output and fails the shape pins).
static FRESH_TEMPLATE: &[u8] = include_bytes!("fresh_template.mdb");

/// Writes a brand-new Jet4 database file. The file must not exist yet;
/// `created` is accepted for call-site stability and stamps nothing — the
/// fresh bytes are the template verbatim (see the module docs for why the
/// creation date is not redated).
pub fn create_jet4_file(path: &Path, _created: f64) -> Result<(), StorageError> {
    datacraft_core::atomic_write::write_new(path, &fresh_bytes()).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => StorageError::Unsupported {
            reason: format!(
                "{}: file already exists — creating would overwrite it",
                path.display()
            ),
        },
        _ => unsupported(path, &e),
    })
}

/// The fresh file's bytes: the template verbatim with the tail's first
/// word normalized (the uniform `0x0100` writer policy; the engine adopts
/// it as its counter baseline on its own sessions — FORMAT-JET).
fn fresh_bytes() -> Vec<u8> {
    let mut file = FRESH_TEMPLATE.to_vec();
    file[TAIL_AT..TAIL_AT + 2].copy_from_slice(&0x0100u16.to_le_bytes());
    file
}

/// Materializes a template — `(definition, seed rows)` per table, in
/// creation order — into a brand-new Jet4 file. Every timestamp (header
/// creation date, catalog rows) is pinned to `created`, so the same
/// template and date produce byte-identical files (LESSONS #6). A failure
/// at any stage removes the partial file.
pub fn materialize_jet4_template(
    path: &Path,
    created: f64,
    tables: &[(TableDef, Vec<Vec<FieldValue>>)],
) -> Result<(), StorageError> {
    let mut writer = Jet4Writer::new(fresh_bytes())?;
    writer.set_catalog_date(created);
    for (definition, rows) in tables {
        let columns = port_write::table_to_columns(definition).map_err(|error| {
            StorageError::Unsupported {
                reason: error.to_string(),
            }
        })?;
        let page = writer.create_table(&definition.name, &columns)?;
        for row in rows {
            let stored =
                port_write::to_storage_values(&definition.fields, row).map_err(|error| {
                    StorageError::Unsupported {
                        reason: error.to_string(),
                    }
                })?;
            writer.insert_row(page, &stored)?;
        }
    }
    datacraft_core::atomic_write::write_replacing(path, &writer.into_file())
        .map_err(|e| unsupported(path, &e))
}

/// Uniform storage error for filesystem failures on a fresh file.
fn unsupported(path: &Path, error: &std::io::Error) -> StorageError {
    StorageError::Unsupported {
        reason: format!("{}: {error}", path.display()),
    }
}
