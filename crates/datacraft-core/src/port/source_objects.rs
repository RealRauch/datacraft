//! Read-only source inventory: indexes and triggers (phase 73, step 73.2;
//! item M90).
//!
//! These describe what the opened file itself carries. DataCraft lists
//! them for users and agents and never creates, drops or executes them
//! (trigger logic stays out of scope, ADR-2 analog).

use serde::{Deserialize, Serialize};

/// One index of the opened source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIndex {
    /// Table the index belongs to.
    pub table: String,
    /// Index name as the file reports it.
    pub name: String,
    /// Indexed columns in declaration order.
    pub columns: Vec<String>,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
    /// Whether the engine created it implicitly (SQLite's
    /// `sqlite_autoindex_*`: a PRIMARY KEY or UNIQUE constraint).
    pub automatic: bool,
}

/// One trigger of the opened source (inventory only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTrigger {
    /// Table the trigger is attached to.
    pub table: String,
    /// Trigger name as the file reports it.
    pub name: String,
    /// The defining statement verbatim.
    pub sql: String,
}

/// Kind of a native Access form/report object (phase 74, step 74.1; item
/// M87). The definitions are proprietary and stay undecoded — the kind is
/// catalog data only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeObjectKind {
    /// A native Access form.
    Form,
    /// A native Access report.
    Report,
}

impl NativeObjectKind {
    /// Lower-case identifier used in snapshot test ids.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Form => "form",
            Self::Report => "report",
        }
    }
}

/// One native Access form/report in the file's catalog (read-only
/// inventory; no definition bytes are decoded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeObject {
    /// Object name as the catalog reports it.
    pub name: String,
    /// Form or report.
    pub kind: NativeObjectKind,
}

/// One driver fact of the opened file (stable machine name + display
/// value): engine version, page size, journal mode, encoding, …
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFact {
    /// Stable fact name (`sqlite_version`, `page_size`, …).
    pub name: String,
    /// The value as text (locale-neutral; the UI formats labels).
    pub value: String,
}

/// Driver facts plus the file's own health verdict (phase 73, step 73.3;
/// item M91).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFacts {
    /// Facts in a stable display order.
    pub facts: Vec<SourceFact>,
    /// `None` = no check ran or it passed; `Some(reason)` = the file's own
    /// bounded health check (`quick_check(1)`) reported damage. The open
    /// still succeeds; every client surfaces the reason.
    pub health: Option<String>,
}
