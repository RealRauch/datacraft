//! Serializable state snapshot (step 4.2, item H14; ARCHITECTURE §5).
//!
//! `Core::inspect` projects the application state into this snapshot —
//! the seam headless tests and agents assert on. Test IDs are composed
//! per the §5 convention; every identifier here is derived, never
//! invented.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::TabId;
use crate::port::DatabaseFormat;
use crate::state::AppState;

/// Snapshot of everything the UI shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStateSnapshot {
    /// The open database, if any.
    pub database: Option<DatabaseSnapshot>,
    /// Navigation pane visibility.
    pub nav_pane_visible: bool,
    /// Open tabs in opening order.
    pub tabs: Vec<TabSnapshot>,
    /// The active tab, if any.
    pub active_tab: Option<TabId>,
}

/// Snapshot of the open database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseSnapshot {
    /// Database name (file stem).
    pub name: String,
    /// Filesystem path of the database file.
    pub path: PathBuf,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// Tables in catalog order, with their navigation-pane test IDs.
    pub tables: Vec<TableEntrySnapshot>,
}

/// One table in the snapshot (navigation pane entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableEntrySnapshot {
    /// Table name as shown in the navigation pane.
    pub name: String,
    /// `navpane.<expanded|collapsed>.<ObjectId::ui_test_id()>`.
    pub ui_test_id: String,
}

/// Snapshot of one open tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSnapshot {
    /// Tab id (stable while the tab is open).
    pub id: TabId,
    /// Object kind slug (`"table"`, `"query"`, …).
    pub object_kind: String,
    /// Object name as shown in the tab bar.
    pub object_name: String,
    /// `workspace.tab.<kind>.<name>`.
    pub ui_test_id: String,
}

impl UiStateSnapshot {
    /// Projects the application state.
    pub fn of(state: &AppState) -> Self {
        Self {
            database: state.database.as_ref().map(|db| DatabaseSnapshot {
                name: db.name.clone(),
                path: db.path.clone(),
                format: db.format,
                tables: db
                    .tables
                    .iter()
                    .map(|t| TableEntrySnapshot {
                        name: t.name.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::Table,
                            &t.name,
                        )),
                    })
                    .collect(),
            }),
            nav_pane_visible: state.nav_pane_visible,
            tabs: state
                .tabs
                .iter()
                .map(|tab| TabSnapshot {
                    id: tab.id,
                    object_kind: tab.object.kind.slug().to_owned(),
                    object_name: tab.object.name.clone(),
                    ui_test_id: tab.ui_test_id(),
                })
                .collect(),
            active_tab: state.active_tab,
        }
    }

    /// Serializes deterministically (pretty JSON, stable field order).
    ///
    /// Infallible for this snapshot: no non-string map keys or other serde
    /// failure sources.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("UiStateSnapshot serialization is infallible")
    }
}
