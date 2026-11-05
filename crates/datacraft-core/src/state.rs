//! Application state (step 4.1, item H13).
//!
//! The reducer's single source of truth; `Core::inspect` (step 4.2)
//! projects it into the serializable snapshot.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::form::FormDef;
use crate::ids::TabId;
use crate::port::{DatabaseFormat, Relationship};
use crate::qbe::QueryDef;
use crate::schema::TableDef;

/// Full application state of the headless core.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AppState {
    /// The open database, if any.
    pub database: Option<DatabaseState>,
    /// Navigation pane visibility (Access default: visible).
    pub nav_pane_visible: bool,
    /// Open document tabs in opening order (tab ids ascend).
    pub tabs: Vec<TabState>,
    /// The active tab, if any.
    pub active_tab: Option<TabId>,
    /// Next tab id the reducer assigns (monotonic, never reused).
    pub next_tab_id: u64,
    /// Whether the relationship mapper is open.
    pub relationship_mapper_open: bool,
    /// Whether the SQL window is open (step 13.3, H37).
    pub sql_window_open: bool,
    /// Whether the expression window is open (step 41.2, item M46).
    pub expression_window_open: bool,
    /// Result of the last successful SQL run (not projected into the
    /// snapshot; the UI renders it from the core's state).
    pub sql_result: Option<crate::sql_types::SqlResult>,
    /// Result of the last successful expression run (step 15.2, H41; not
    /// projected into the snapshot — agents read it via `Core`).
    pub expression_result: Option<crate::schema::FieldValue>,
    /// Next confirmation token the journal assigns (monotonic).
    pub next_write_token: u64,
    /// Previewed write batch awaiting confirmation (step 11.11, M2).
    pub pending_write: Option<crate::journal::PendingWrite>,
    /// Last applied write batch, kept for rollback (step 11.11, M2).
    pub last_applied: Option<crate::journal::AppliedWrite>,
    /// The composition's driver inventory (step 36.4, M23) — fixed at
    /// core construction from the port's registry; never mutates.
    pub drivers: Vec<crate::port::DriverInfo>,
}

impl AppState {
    /// Initial state: empty, navigation pane visible, tab ids 1-based.
    pub fn new() -> Self {
        Self {
            nav_pane_visible: true,
            next_tab_id: 1,
            next_write_token: 1,
            ..Self::default()
        }
    }

    /// State-independent test id of a navigation-pane node, composed from
    /// the object id per the ARCHITECTURE §5 convention.
    pub fn nav_pane_test_id(&self, object: &ObjectId) -> String {
        let state = if self.nav_pane_visible {
            "expanded"
        } else {
            "collapsed"
        };
        format!("navpane.{}.{}", state, object.ui_test_id())
    }
}

/// State of the open database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseState {
    /// Per-table datasheet view state (step 44.5, item M54): sort and
    /// column filters, keyed by table name (case-insensitive keys are
    /// written by the dispatch validation).
    pub table_views: std::collections::BTreeMap<String, TableViewState>,
    /// Filesystem path of the database file.
    pub path: PathBuf,
    /// Database name (file stem).
    pub name: String,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// Identity of the driver serving this source (phase 30, M17; ADR-7).
    pub driver: crate::port::DriverId,
    /// The driver's per-session capabilities (phase 30, M17; ADR-7).
    pub capabilities: crate::port::DriverCapabilities,
    /// Table definitions in catalog order.
    pub tables: Vec<TableDef>,
    /// Referential relationships, as reported by the backend.
    pub relationships: Vec<Relationship>,
    /// Query definitions in creation order.
    pub queries: Vec<QueryDef>,
    /// Form design documents in creation order.
    pub forms: Vec<FormDef>,
    /// Report design documents in creation order (step 10.4, H31).
    pub reports: Vec<crate::report::ReportDef>,
    /// Native saved queries read from the file (step 15.3, M3; read-only).
    pub native_queries: Vec<crate::port::NativeQuery>,
    /// Why the native-query decode failed, if it did — the open still
    /// succeeds when the tables are readable (step 23.3, M10).
    pub native_queries_error: Option<String>,
    /// Named macros read from the file (step 17.1, L4; read-only).
    pub macros: Vec<crate::macro_def::MacroDef>,
    /// Why the macro inventory could not be decoded, if it failed — the
    /// open still succeeds when the tables are readable.
    pub macros_error: Option<String>,
    /// Extracted VBA project (step 17.2, L5; read-only, never executed).
    pub vba_project: Option<crate::vba::VbaProject>,
    /// Why the VBA extraction failed, if it did — the open still succeeds.
    pub vba_error: Option<String>,
    /// Why the sibling `.datacraft` sidecar did not auto-load (phase 29,
    /// step 29.4): broken JSON or a foreign database name. A missing
    /// sidecar is not an error.
    pub sidecar_error: Option<String>,
    /// Row-write history, oldest first (step 28.1, item U14). Bounded by
    /// [`crate::undo::MAX_HISTORY`]; schema and batch writes clear it.
    #[serde(default)]
    pub undo: Vec<crate::undo::HistoryEntry>,
    /// Undone row writes, most recently undone last (step 28.1, U14).
    #[serde(default)]
    pub redo: Vec<crate::undo::HistoryEntry>,
}

/// View mode of one document tab (step 21.1, item U5) — document state,
/// not a UI setting (ADR-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TabMode {
    /// The datasheet/data view (the default every tab opens in).
    #[default]
    Datasheet,
    /// The design view.
    Design,
}

impl TabMode {
    /// The other view — the Views split-button's primary action.
    pub const fn toggled(self) -> Self {
        match self {
            TabMode::Datasheet => TabMode::Design,
            TabMode::Design => TabMode::Datasheet,
        }
    }
}

/// Datasheet view state of one table (step 44.5, item M54): the sort
/// (field + direction) and the column filters (field → criteria text).
/// Presentation model — the row data itself stays untouched.
/// (field + direction) and the column filters (field → criteria text).
/// Presentation model — the row data itself stays untouched.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableViewState {
    /// Sort field name (whitelisted at set time), if any.
    pub sort_field: Option<String>,
    /// Sort direction, if a sort is set.
    pub sort_order: Option<crate::qbe::SortOrder>,
    /// Column filters: field name → criteria text (engine dialect).
    pub filters: std::collections::BTreeMap<String, String>,
}

/// State of one open document tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabState {
    /// Tab id (stable while the tab is open).
    pub id: TabId,
    /// The object shown in the tab.
    pub object: ObjectId,
    /// Current view mode (step 21.1, U5).
    #[serde(default)]
    pub mode: TabMode,
}

impl TabState {
    /// State-independent test id of the tab, composed per the
    /// ARCHITECTURE §5 convention: `workspace.tab.<kind>.<name>`.
    pub fn ui_test_id(&self) -> String {
        format!(
            "workspace.tab.{}.{}",
            self.object.kind.slug(),
            self.object.name.to_lowercase()
        )
    }
}
