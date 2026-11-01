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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiStateSnapshot {
    /// The open database, if any.
    pub database: Option<DatabaseSnapshot>,
    /// Navigation pane visibility.
    pub nav_pane_visible: bool,
    /// Open tabs in opening order.
    pub tabs: Vec<TabSnapshot>,
    /// The active tab, if any.
    pub active_tab: Option<TabId>,
    /// Whether the relationship mapper is open.
    pub relationship_mapper_open: bool,
    /// Whether the SQL window is open (step 13.3, H37).
    pub sql_window_open: bool,
}

/// Snapshot of the open database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseSnapshot {
    /// Database name (file stem).
    pub name: String,
    /// Filesystem path of the database file.
    pub path: PathBuf,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// Tables in catalog order, with their navigation-pane test IDs.
    pub tables: Vec<TableEntrySnapshot>,
    /// Referential relationships (mapper pane and agents).
    pub relationships: Vec<crate::port::Relationship>,
    /// Query definitions in creation order.
    pub queries: Vec<QueryEntrySnapshot>,
    /// Form design documents in creation order.
    pub forms: Vec<crate::form::FormDef>,
    /// Report design documents in creation order (step 10.4, H31).
    pub reports: Vec<crate::report::ReportDef>,
}

/// One query in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryEntrySnapshot {
    /// Query name.
    pub name: String,
    /// Source table name.
    pub table: String,
    /// `navpane.<expanded|collapsed>.node.query.<name>`.
    pub ui_test_id: String,
    /// QBE columns.
    pub fields: Vec<crate::qbe::QbeField>,
}

/// One field of a table in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSnapshot {
    /// Field name.
    pub name: String,
    /// Domain kind (designer field-list mapping and agents).
    pub kind: crate::schema::FieldKind,
    /// Access UI type name (design grid Data Type column).
    pub kind_display: String,
    /// Full field properties (design grid property sheet).
    pub properties: crate::schema::FieldProperties,
}

/// One table in the snapshot (navigation pane entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableEntrySnapshot {
    /// Table name as shown in the navigation pane.
    pub name: String,
    /// `navpane.<expanded|collapsed>.<ObjectId::ui_test_id()>`.
    pub ui_test_id: String,
    /// Fields in storage order (datasheet headers render from this).
    pub fields: Vec<FieldSnapshot>,
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
                relationships: db.relationships.clone(),
                forms: db.forms.clone(),
                reports: db.reports.clone(),
                queries: db
                    .queries
                    .iter()
                    .map(|q| QueryEntrySnapshot {
                        name: q.name.clone(),
                        table: q.table.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::Query,
                            &q.name,
                        )),
                        fields: q.fields.clone(),
                    })
                    .chain(db.native_queries.iter().map(|query| QueryEntrySnapshot {
                        name: query.name.clone(),
                        table: query.tables.first().cloned().unwrap_or_default(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::Query,
                            &query.name,
                        )),
                        fields: query.query_def().map(|def| def.fields).unwrap_or_default(),
                    }))
                    .collect(),
                tables: db
                    .tables
                    .iter()
                    .map(|t| TableEntrySnapshot {
                        name: t.name.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::Table,
                            &t.name,
                        )),
                        fields: t
                            .fields
                            .iter()
                            .map(|f| FieldSnapshot {
                                name: f.name.clone(),
                                kind: f.kind,
                                kind_display: crate::present::field_kind_display(&f.kind),
                                properties: f.properties.clone(),
                            })
                            .collect(),
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
            relationship_mapper_open: state.relationship_mapper_open,
            sql_window_open: state.sql_window_open,
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
