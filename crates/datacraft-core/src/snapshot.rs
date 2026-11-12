//! Serializable state snapshot (step 4.2, item H14; ARCHITECTURE §5).
//!
//! `Core::inspect` projects the application state into this snapshot —
//! the seam headless tests and agents assert on. Test IDs are composed
//! per the §5 convention; every identifier here is derived, never
//! invented.

use std::collections::BTreeMap;
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
    /// Whether the Import File window is open (step 58.1, item M50).
    pub import_window_open: bool,
    /// The Import File window's parsed preview, if a source was loaded
    /// (step 58.1, item M50).
    pub import_preview: Option<crate::import::ImportPreview>,
    /// Whether the expression window is open (step 41.2, item M46).
    pub expression_window_open: bool,
    /// The shipped database templates (phase 29, item H44) — the registry
    /// behind `CreateDatabaseFromTemplate`, visible to every client.
    pub templates: Vec<crate::templates::TemplateSummary>,
    /// The composition's driver inventory (step 36.4, M23): every driver
    /// the composed binary can open with id, fixed family and offline
    /// capability summary — additive next to `templates`, so agents
    /// pre-flight an open against the reported families.
    pub drivers: Vec<crate::port::DriverInfo>,
    /// Row-write undo depth of the open database (step 28.1, item U14);
    /// `0` without a database.
    pub undo_depth: usize,
    /// Row-write redo depth of the open database (step 28.1, item U14).
    pub redo_depth: usize,
}

/// Snapshot of the open database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseSnapshot {
    /// Database name (file stem).
    pub name: String,
    /// Filesystem path of the database file.
    pub path: PathBuf,
    /// The source spec the session was opened with (step 65.2, item L8):
    /// `None` for plain path opens; clients render it beside `path`
    /// (wire-additive — older snapshot JSON without the key keeps
    /// deserializing to `None`).
    #[serde(default)]
    pub source: Option<crate::command::SourceSpec>,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// Identity of the driver serving this source (phase 30, M17;
    /// ADR-7) — GUI, CLI and MCP agents see the same value.
    pub driver: crate::port::DriverId,
    /// The driver's per-session capabilities (phase 30, M17; ADR-7).
    pub capabilities: crate::port::DriverCapabilities,
    /// Disabled command affordances for this source with their reasons
    /// (phase 30, M17; ADR-7): absence means enabled, so capable sources
    /// carry an empty map. UI code renders from this — never from format
    /// matches.
    pub availability: BTreeMap<String, crate::availability::DisabledReason>,
    /// Tables in catalog order, with their navigation-pane test IDs.
    pub tables: Vec<TableEntrySnapshot>,
    /// Referential relationships (mapper pane and agents). System rows
    /// (`MSys*`) are filtered at load — they are Access's internal
    /// metadata, not user relationships (step 69.1, H57).
    pub relationships: Vec<crate::port::Relationship>,
    /// Query definitions in creation order.
    pub queries: Vec<QueryEntrySnapshot>,
    /// Sidecar view designs in creation order (step 71.1, item M93).
    pub views: Vec<ViewEntrySnapshot>,
    /// File-owned views of the opened source (phase 73, step 73.1; M90) —
    /// read-only; sidecar designs with the same name are not repeated.
    #[serde(default)]
    pub native_views: Vec<NativeViewSnapshot>,
    /// Indexes the opened source carries (phase 73, step 73.2; M90) —
    /// read-only inventory.
    #[serde(default)]
    pub indexes: Vec<crate::port::SourceIndex>,
    /// Triggers the opened source carries (phase 73, step 73.2; M90) —
    /// inventory only.
    #[serde(default)]
    pub triggers: Vec<crate::port::SourceTrigger>,
    /// Driver facts of the opened file (phase 73, step 73.3; M91).
    #[serde(default)]
    pub facts: Vec<crate::port::SourceFact>,
    /// The file's health verdict: `None` = passed or not checked,
    /// `Some(reason)` = the bounded check reported damage (M91).
    #[serde(default)]
    pub health: Option<String>,
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
    /// A sibling Access lock file detected at open (step 53.1, item M59):
    /// the file NAME as data — the session opened read-only. Clients
    /// localize the notice from this field (ADR-4); absent when no lock
    /// file exists beside the database.
    pub lock_file: Option<String>,
}

/// One file-owned (native) view in the snapshot (phase 73, step 73.1;
/// item M90). Read-only: no design tab, no write affordance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeViewSnapshot {
    /// View name as the file reports it.
    pub name: String,
    /// The source statement verbatim (the tab's boundary hint).
    pub sql: String,
    /// `navpane.<expanded|collapsed>.node.view.<name>`.
    pub ui_test_id: String,
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

/// One sidecar view in the snapshot (step 71.1, item M93).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEntrySnapshot {
    /// View name.
    pub name: String,
    /// The SELECT statement (agents and the view tab execute it).
    pub sql: String,
    /// `navpane.<expanded|collapsed>.node.view.<name>`.
    pub ui_test_id: String,
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
    /// Datasheet sort field of a table tab (step 44.5, item M54).
    pub sort_field: Option<String>,
    /// Datasheet sort direction of a table tab (step 44.5, item M54).
    pub sort_order: Option<crate::qbe::SortOrder>,
    /// Datasheet column filters of a table tab (step 44.5, item M54):
    /// field name → criteria text.
    pub filters: std::collections::BTreeMap<String, String>,
}

impl UiStateSnapshot {
    /// Projects the application state.
    pub fn of(state: &AppState) -> Self {
        Self {
            templates: crate::templates::summaries(),
            drivers: state.drivers.clone(),
            database: state.database.as_ref().map(|db| DatabaseSnapshot {
                name: db.name.clone(),
                path: db.path.clone(),
                source: db.source.clone(),
                format: db.format,
                driver: db.driver.clone(),
                capabilities: db.capabilities,
                availability: crate::availability::affordances(&db.capabilities),
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
                lock_file: db.lock_file.clone(),
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
                indexes: db.indexes.clone(),
                triggers: db.triggers.clone(),
                facts: db.source_facts.facts.clone(),
                health: db.source_facts.health.clone(),
                native_views: db
                    .native_views
                    .iter()
                    .map(|view| NativeViewSnapshot {
                        name: view.name.clone(),
                        sql: view.sql.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::View,
                            &view.name,
                        )),
                    })
                    .collect(),
                views: db
                    .views
                    .iter()
                    .map(|view| ViewEntrySnapshot {
                        name: view.name.clone(),
                        sql: view.sql.clone(),
                        ui_test_id: state.nav_pane_test_id(&crate::ObjectId::new(
                            crate::ObjectKind::View,
                            &view.name,
                        )),
                    })
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
                .map(|tab| {
                    let view = state
                        .database
                        .as_ref()
                        .and_then(|db| db.table_views.get(&tab.object.name))
                        .filter(|_| tab.object.kind == crate::ObjectKind::Table);
                    TabSnapshot {
                        id: tab.id,
                        object_kind: tab.object.kind.slug().to_owned(),
                        object_name: tab.object.name.clone(),
                        ui_test_id: tab.ui_test_id(),
                        mode: tab.mode,
                        has_design_view: tab.object.kind.has_design_view(),
                        sort_field: view.and_then(|view| view.sort_field.clone()),
                        sort_order: view.and_then(|view| view.sort_order),
                        filters: view.map(|view| view.filters.clone()).unwrap_or_default(),
                    }
                })
                .collect(),
            active_tab: state.active_tab,
            relationship_mapper_open: state.relationship_mapper_open,
            sql_window_open: state.sql_window_open,
            import_window_open: state.import_window_open,
            import_preview: state.import_preview.clone(),
            expression_window_open: state.expression_window_open,
            undo_depth: state.database.as_ref().map_or(0, |db| db.undo.len()),
            redo_depth: state.database.as_ref().map_or(0, |db| db.redo.len()),
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
