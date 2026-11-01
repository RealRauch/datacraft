//! Presentation helpers (step 5.1, item H17; ARCHITECTURE §8).
//!
//! The UI renders strings and models computed here so every piece of
//! presentation logic is unit-testable without a window; paint code stays
//! free of layout or business logic.

use crate::ObjectKind;
use crate::port::DatabaseFormat;
use crate::snapshot::UiStateSnapshot;

/// Status bar line for the given snapshot.
pub fn status_line(snapshot: &UiStateSnapshot) -> String {
    match &snapshot.database {
        None => "No database open".to_owned(),
        Some(db) => {
            let count = db.tables.len();
            let noun = if count == 1 { "table" } else { "tables" };
            format!("{} ({}) — {count} {noun}", db.name, db.format)
        }
    }
}

impl std::fmt::Display for DatabaseFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Jet3 => write!(f, "Jet3"),
            Self::Jet4 => write!(f, "Jet4"),
            Self::Ace => write!(f, "Ace"),
        }
    }
}

/// Central-area placeholder text for an open tab (the real views land with
/// their phases: datasheet 6, query results 7, form/report views 8).
pub fn tab_placeholder(kind: ObjectKind, name: &str) -> String {
    match kind {
        ObjectKind::Table => format!("Datasheet for {name:?} arrives in Phase 6."),
        ObjectKind::Query => format!("Query results for {name:?} arrive in Phase 7."),
        ObjectKind::Form => format!("Form view for {name:?} arrives in Phase 8."),
        ObjectKind::Report => format!("Report preview for {name:?} arrives in Phase 8."),
    }
}
