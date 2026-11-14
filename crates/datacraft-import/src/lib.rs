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

pub mod cli;

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

/// Merges the imported forms/reports into an existing sidecar document:
/// same-named objects (case-insensitive) are replaced, everything else —
/// including tables, queries, views and unrelated designs — is kept.
pub fn merge_documents(into: &mut SchemaDocument, imported: SchemaDocument) {
    merge_by_name(&mut into.forms, imported.forms, |form| &form.name);
    merge_by_name(&mut into.reports, imported.reports, |report| &report.name);
}

fn merge_by_name<T>(target: &mut Vec<T>, items: Vec<T>, name: impl Fn(&T) -> &str) {
    for item in items {
        let key = name(&item).to_lowercase();
        if let Some(slot) = target
            .iter_mut()
            .find(|existing| name(existing).to_lowercase() == key)
        {
            *slot = item;
        } else {
            target.push(item);
        }
    }
}

/// Whether any object carried an unsupported control or property.
pub fn has_unsupported(report: &ImportReport) -> bool {
    report.objects.iter().any(|object| {
        !object.unsupported_controls.is_empty() || !object.unsupported_properties.is_empty()
    })
}

#[cfg(test)]
mod tests {
    use datacraft_core::form::FormDef;
    use datacraft_core::report::ReportDef;
    use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

    use super::*;
    use crate::report::UnsupportedProperty;

    fn document(forms: &[&str], reports: &[&str]) -> SchemaDocument {
        let mut document = SchemaDocument::new("db");
        document.forms = forms.iter().map(|name| FormDef::new(*name)).collect();
        document.reports = reports.iter().map(|name| ReportDef::new(*name)).collect();
        document
    }

    #[test]
    fn merge_replaces_by_name_and_keeps_unrelated_content() {
        let mut existing = document(&["Items Browser", "Keep Me"], &["Old Report"]);
        existing.tables = vec![TableDef::new(
            "Customers",
            vec![FieldDef::new(
                "ID",
                FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
            )],
        )];
        let imported = document(
            &["items browser", "New Form"],
            &["Old Report", "New Report"],
        );
        merge_documents(&mut existing, imported);
        let names: Vec<&str> = existing
            .forms
            .iter()
            .map(|form| form.name.as_str())
            .collect();
        assert_eq!(names, ["items browser", "Keep Me", "New Form"]);
        let reports: Vec<&str> = existing
            .reports
            .iter()
            .map(|report| report.name.as_str())
            .collect();
        assert_eq!(reports, ["Old Report", "New Report"]);
        assert_eq!(existing.tables.len(), 1, "tables are untouched");
    }

    #[test]
    fn has_unsupported_reads_controls_and_properties() {
        let clean = ImportReport::default();
        assert!(!has_unsupported(&clean));
        let mut report = ImportReport::default();
        report.objects.push(ObjectReport {
            object: "F".to_owned(),
            kind: "form".to_owned(),
            imported_controls: 0,
            unsupported_controls: Vec::new(),
            unsupported_properties: vec![UnsupportedProperty {
                control: None,
                property: "RecordSource".to_owned(),
                reason: "not plain".to_owned(),
            }],
            ignored_properties: Vec::new(),
        });
        assert!(has_unsupported(&report));
    }
}
