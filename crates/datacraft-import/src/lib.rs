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

use datacraft_core::schema::SchemaDocument;
use jetdb::{PageReader, list_forms, read_form_properties_of_type, read_queries, table_names};

mod error;
mod map;
mod report;

pub use error::ImportError;
pub use report::{ImportReport, ObjectReport, UnsupportedControl, UnsupportedProperty};

/// Result of one import run: the sidecar document plus the per-object report.
#[derive(Debug)]
pub struct Import {
    /// The sidecar document built from the database's forms and reports.
    pub document: SchemaDocument,
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
    let mut reader = PageReader::open(database)?;
    let tables = table_names(&mut reader)?;
    let queries: Vec<String> = read_queries(&mut reader)?
        .into_iter()
        .map(|query| query.name)
        .collect();

    let mut entries = list_forms(&mut reader)?;
    entries.sort_by_key(|entry| entry.name.to_lowercase());

    let mut document = SchemaDocument::new(name);
    let mut report = ImportReport::default();
    for entry in entries {
        let properties = read_form_properties_of_type(&mut reader, &entry.name, entry.object_type)?;
        let raw = map::RawObject::from_jetdb(properties);
        let (design, object_report) = map::map_object(&raw, &tables, &queries);
        match design {
            Some(map::Design::Form(form)) => document.forms.push(form),
            Some(map::Design::Report(report_design)) => document.reports.push(report_design),
            None => {}
        }
        report.objects.push(object_report);
    }
    Ok(Import { document, report })
}
