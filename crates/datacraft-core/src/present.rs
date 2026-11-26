//! Presentation helpers (step 5.1, item H17; ARCHITECTURE §8).
//!
//! The UI renders strings and models computed here so every piece of
//! presentation logic is unit-testable without a window; paint code stays
//! free of layout or business logic. The pure value-rendering helpers live
//! in [`datacraft_types`] and are re-exported here unchanged (phase 76,
//! item L36); this module keeps the helpers that need the command layer.

use crate::port::DatabaseFormat;
use crate::snapshot::UiStateSnapshot;

pub use datacraft_types::{display_value, field_kind_display};

/// The one-line target a linked table names (phase 100, step 100.3; item
/// M131): the external file of an Access-file link, or the first fragment
/// of an ODBC connect string (`DSN=…`, `DRIVER=…`). The connect string
/// arrives credential-free from the port, so nothing here can leak a
/// secret; an empty fragment set answers `None`.
pub fn link_target(link: &crate::port::LinkedTable) -> Option<String> {
    match link.kind {
        crate::port::LinkKind::AccessFile => link.source.clone(),
        crate::port::LinkKind::Odbc => link.connect.as_deref().and_then(|connect| {
            connect
                .split(';')
                .map(str::trim)
                .find(|fragment| !fragment.is_empty())
                .map(str::to_owned)
        }),
    }
}

/// Status bar line for the given snapshot (step 12.4): the database
/// context, replaced by the active document's object when one is open —
/// the Access status-bar convention. `active` carries the tab snapshot's
/// `(object_kind, object_name)`.
pub fn status_line(snapshot: &UiStateSnapshot, active: Option<(&str, &str)>) -> String {
    let Some(db) = &snapshot.database else {
        return "No database open".to_owned();
    };
    match active {
        Some((_, name)) => format!("{} ({}) — {name}", db.name, db.format),
        None => {
            let count = db.tables.len();
            let noun = if count == 1 { "table" } else { "tables" };
            format!("{} ({}) — {count} {noun}", db.name, db.format)
        }
    }
}

impl std::fmt::Display for DatabaseFormat {
    /// The single rendering point for format families: the window title,
    /// the status bar and `status_line` all render through this path, so
    /// a new variant adds exactly one arm here (phase 30, item M17;
    /// ADR-7 data-driven rendering).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Jet3 => write!(f, "Jet3"),
            Self::Jet4 => write!(f, "Jet4"),
            Self::Ace => write!(f, "ACE"),
            Self::Sqlite => write!(f, "SQLite"),
            Self::Csv => write!(f, "CSV"),
            Self::Excel => write!(f, "Excel"),
        }
    }
}

/// Access toolbox name of a control kind (the designer's palette label).
pub fn control_kind_display(kind: &crate::form::ControlKind) -> &'static str {
    match kind {
        crate::form::ControlKind::Label { .. } => "Label",
        crate::form::ControlKind::TextBox { .. } => "Text Box",
        crate::form::ControlKind::CheckBox { .. } => "Check Box",
        crate::form::ControlKind::ComboBox { .. } => "Combo Box",
        crate::form::ControlKind::Button { .. } => "Button",
        crate::form::ControlKind::SubForm { .. } => "Subform",
    }
}
