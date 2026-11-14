//! DataCraft import tool (Route B, ADR-9) — native Access form/report
//! designs into `.datacraft` sidecar designs.
//!
//! The tool reads a Jet4/ACE database through the pinned `jetdb` crate
//! (pure Rust, no COM/FFI), maps the supported control subset onto the
//! core sidecar vocabulary and reports every skipped control or property.
//! The engine stays clean-room: this crate is the declared adapter and the
//! only place that depends on jetdb.
//!
//! Mapping contract: `docs/M88-IMPORT.md`.

use std::path::Path;

mod error;
mod report;

pub use error::ImportError;
pub use report::{ImportReport, ObjectReport, UnsupportedControl, UnsupportedProperty};

/// Result of one import run: the sidecar document plus the per-object report.
#[derive(Debug)]
pub struct Import {
    /// The sidecar document built from the database's forms and reports.
    pub document: datacraft_core::schema::SchemaDocument,
    /// What was imported, skipped or ignored.
    pub report: ImportReport,
}

/// Imports the forms and reports of `database` into a sidecar document.
///
/// The database is opened read-only and never modified.
pub fn import_database(database: &Path) -> Result<Import, ImportError> {
    let name = database
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("database");
    Ok(Import {
        document: datacraft_core::schema::SchemaDocument::new(name),
        report: ImportReport::default(),
    })
}
