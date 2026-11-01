//! Application state (step 4.1, item H13).
//!
//! The reducer's single source of truth; `Core::inspect` (step 4.2)
//! projects it into the serializable snapshot.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::ids::TabId;
use crate::port::DatabaseFormat;
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
}

impl AppState {
    /// Initial state: empty, navigation pane visible, tab ids 1-based.
    pub fn new() -> Self {
        Self {
            nav_pane_visible: true,
            next_tab_id: 1,
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
    /// Filesystem path of the database file.
    pub path: PathBuf,
    /// Database name (file stem).
    pub name: String,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// Table definitions in catalog order.
    pub tables: Vec<TableDef>,
}

/// State of one open document tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabState {
    /// Tab id (stable while the tab is open).
    pub id: TabId,
    /// The object shown in the tab.
    pub object: ObjectId,
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
