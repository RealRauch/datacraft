//! Snapshot entry structs (split from `snapshot.rs` for the file-size gate):
//! the per-object projections the UI and agents read.

use serde::{Deserialize, Serialize};

use crate::ids::TabId;

/// One linked table in the snapshot (phase 100, step 100.2; item M131;
/// roadmap §9 stage 0). Read-only: the record is catalog data, no
/// connection is opened and the target is never resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedTableSnapshot {
    /// Local table name as the catalog reports it.
    pub name: String,
    /// ODBC or Access-file link.
    pub kind: crate::port::LinkKind,
    /// External file of an Access-file link; `None` for an ODBC link.
    pub source: Option<String>,
    /// The ODBC connect string with its credentials redacted; `None` for
    /// an Access-file link.
    pub connect: Option<String>,
    /// Name on the remote side.
    pub foreign_name: Option<String>,
    /// `MSysObjects.Flags` of the link record.
    pub flags: i64,
    /// `navpane.<expanded|collapsed>.linked.<kind>.<name>`.
    pub ui_test_id: String,
}

/// One native Access form/report in the snapshot (phase 74, step 74.1;
/// item M87). Read-only: no tab, no definition bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeObjectSnapshot {
    /// Object name as the catalog reports it.
    pub name: String,
    /// Form or report.
    pub kind: crate::port::NativeObjectKind,
    /// `navpane.<expanded|collapsed>.native.<kind>.<name>`.
    pub ui_test_id: String,
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
