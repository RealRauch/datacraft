//! Shipped database templates (phase 29, item H44).
//!
//! Templates are core-owned, deterministic definitions: every table and
//! row is fixed data, so a template materializes identically everywhere —
//! the sample generator pins the bytes with `--check` (step 29.3). The
//! registry grows with the shipped samples; `blank` is the base entry.

use serde::{Deserialize, Serialize};

use crate::port::DatabaseFormat;
use crate::schema::{FieldValue, TableDef};

/// One table of a template: its definition plus the seed rows.
#[derive(Debug, Clone)]
pub struct TemplateTable {
    /// Table definition (field kinds, properties).
    pub definition: TableDef,
    /// Seed rows in insertion order; `Null` on an AutoNumber field means
    /// engine-assigned.
    pub rows: Vec<Vec<FieldValue>>,
}

/// A shipped database template.
#[derive(Debug, Clone)]
pub struct DatabaseTemplate {
    /// Registry name (the command's `template` argument).
    pub name: &'static str,
    /// Human-facing title.
    pub title: &'static str,
    /// Format family of the materialized file.
    pub format: DatabaseFormat,
    /// Tables in creation order.
    pub tables: Vec<TemplateTable>,
}

/// Serializable template summary for snapshots (the wire-visible registry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSummary {
    /// Registry name.
    pub name: String,
    /// Human-facing title.
    pub title: String,
    /// Format family of the materialized file.
    pub format: DatabaseFormat,
    /// Number of tables the template creates.
    pub tables: usize,
}

/// The shipped template registry (order = backstage listing order).
pub fn templates() -> Vec<DatabaseTemplate> {
    vec![blank()]
}

/// Resolves a template by its registry name (case-insensitive).
pub fn find_template(name: &str) -> Option<DatabaseTemplate> {
    templates()
        .into_iter()
        .find(|template| template.name.eq_ignore_ascii_case(name))
}

/// Snapshot summaries of every shipped template.
pub fn summaries() -> Vec<TemplateSummary> {
    templates()
        .into_iter()
        .map(|template| TemplateSummary {
            name: template.name.to_owned(),
            title: template.title.to_owned(),
            format: template.format,
            tables: template.tables.len(),
        })
        .collect()
}

/// The empty database: a fresh file with no tables.
fn blank() -> DatabaseTemplate {
    DatabaseTemplate {
        name: "blank",
        title: "Blank database",
        format: DatabaseFormat::Jet4,
        tables: Vec::new(),
    }
}
