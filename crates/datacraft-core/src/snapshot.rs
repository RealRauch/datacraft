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
    /// The shipped database templates (phase 29, item H44) — the registry
    /// behind `CreateDatabaseFromTemplate`, visible to every client.
    pub templates: Vec<crate::templates::TemplateSummary>,
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
    /// Identity of the driver serving this source (phase 30, M17;
    /// ADR-7) — GUI, CLI and MCP agents see the same value.
    pub driver: crate::port::DriverId,
    /// The driver's per-session capabilities (phase 30, M17; ADR-7).
    pub capabilities: crate::port::DriverCapabilities,
    /// Tables in catalog order, with their navigation-pane test IDs.
    pub tables: Vec<TableEntrySnapshot>,
    /// Referential relationships (mapper pane and agents).
    pub relationships: Vec<crate::port::Relationship>,
    /// Query definitions in creation order.
    pub queries: Vec<QueryEntrySnapshot>,
    /// Why the native-query decode failed, if it did (step 23.3, M10).
    pub native_queries_error: Option<String>,
    /// Form design documents in creation order.
    pub forms: Vec<crate::form::FormDef>,
    /// Report design documents in creation order (step 10.4, H31).
    pub reports: Vec<crate::report::ReportDef>,
    /// Named macros in catalog order (step 17.1, L4; read-only inventory).
    pub macros: Vec<MacroEntrySnapshot>,
    /// Why the macro inventory could not be decoded, if it failed.
    pub macros_error: Option<String>,
    /// Extracted VBA project summary (step 17.2, L5); `None` when the
    /// database has no decoded project.
    pub vba: Option<VbaProjectSnapshot>,
    /// Why the VBA extraction failed, if it did.
    pub vba_error: Option<String>,
    /// Why the sibling `.datacraft` sidecar did not auto-load (phase 29,
    /// step 29.4); absent for a missing sidecar.
    pub sidecar_error: Option<String>,
}

/// Snapshot of the extracted VBA project (module tree only — source is
/// fetched per module through `LoadVbaModule`, never mirrored here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaProjectSnapshot {
    /// Modules in `dir`-stream order.
    pub modules: Vec<VbaModuleSnapshot>,
}

/// One VBA module in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaModuleSnapshot {
    /// Module name as shown in the navigation pane and viewer.
    pub name: String,
    /// `navpane.<expanded|collapsed>.node.module.<name>`.
    pub ui_test_id: String,
    /// Module kind.
    pub kind: crate::vba::VbaModuleKind,
    /// Source line count (the viewer's summary).
    pub lines: usize,
}

/// One macro in the snapshot (navigation pane entry and viewer input).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroEntrySnapshot {
    /// Macro name as shown in the navigation pane.
    pub name: String,
    /// `navpane.<expanded|collapsed>.node.macro.<name>`.
    pub ui_test_id: String,
    /// Readable definition arguments in storage order.
    pub actions: Vec<crate::macro_def::MacroAction>,
    /// Whether the definition blob was decoded.
    pub definition_decoded: bool,
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
    /// Current view mode (step 21.1, item U5).
    pub mode: crate::state::TabMode,
    /// Whether the tab has a second (design) view (step 21.1, item U5).
    pub has_design_view: bool,
}

impl UiStateSnapshot {
    /// Projects the application state.
    pub fn of(state: &AppState) -> Self {
        Self {
            templates: crate::templates::summaries(),
            database: state.database.as_ref().map(|db| DatabaseSnapshot {
                name: db.name.clone(),
                path: db.path.clone(),
                format: db.format,
                driver: db.driver.clone(),
                capabilities: db.capabilities,
                relationships: db.relationships.clone(),
                forms: db.forms.clone(),
                reports: db.reports.clone(),
                macros: db
                    .macros
                    .iter()
                    .map(|macro_def| MacroEntrySnapshot {
                        name: macro_def.name.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::Macro,
                            &macro_def.name,
                        )),
                        actions: macro_def.actions.clone(),
                        definition_decoded: macro_def.definition_decoded,
                    })
                    .collect(),
                macros_error: db.macros_error.clone(),
                native_queries_error: db.native_queries_error.clone(),
                vba: db.vba_project.as_ref().map(|project| VbaProjectSnapshot {
                    modules: project
                        .modules
                        .iter()
                        .map(|module| VbaModuleSnapshot {
                            name: module.name.clone(),
                            ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                                crate::ObjectKind::Module,
                                &module.name,
                            )),
                            kind: module.kind,
                            lines: module.lines(),
                        })
                        .collect(),
                }),
                vba_error: db.vba_error.clone(),
                sidecar_error: db.sidecar_error.clone(),
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
                    mode: tab.mode,
                    has_design_view: tab.object.kind.has_design_view(),
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
