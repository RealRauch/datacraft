//! DataCraft core — headless domain model, command bus and application state.
//!
//! The GUI ([`datacraft-ui`](https://docs.rs/datacraft-ui)) and future
//! MCP/JSON-RPC endpoints are thin clients of this crate: every user action
//! maps to one command, every observable outcome is queryable as serializable
//! state. See `docs/ARCHITECTURE.md` §5.

pub mod atomic_write;
pub mod availability;
pub mod canvas;
pub mod command;
pub mod core;
mod database;
pub mod event;
pub use datacraft_expr as expr;
pub mod failure;
pub mod field_expressions;
pub mod form;
mod forms;
pub mod grid;
pub mod ids;
pub mod import;
pub mod journal;
pub mod keys;
pub mod macro_def;
pub use datacraft_query::native_exec;
pub mod nav;
pub mod palette;
pub mod port;
mod port_mock;
pub mod present;
pub use datacraft_query::qbe;
mod qbe_dispatch;
pub use datacraft_query::query_criteria;
pub use datacraft_query::query_engine;
pub mod report;
pub mod report_aggregates;
pub mod report_bands;
pub mod report_preview;
mod reports;
pub mod ribbon;
pub mod row_edit;
pub mod schema;
pub mod snapshot;
#[cfg(feature = "sql-engine")]
pub use datacraft_query::sql_engine;
pub use datacraft_query::sql_types;
pub mod state;
pub mod table_view;
pub mod templates;
pub mod transport;
pub mod undo;
pub mod vba;
pub mod vba_analysis;
mod vba_dispatch;
pub mod view;

pub use ids::TabId;

use serde::{Deserialize, Serialize};

/// Kind of a database object, following the Access object paradigm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    /// A table (datasheet and design view).
    Table,
    /// A saved query (QBE grid).
    Query,
    /// A sidecar view design: a named SELECT (step 71.1, item M93).
    View,
    /// A form design (canvas and runtime view).
    Form,
    /// A report design (canvas and print preview).
    Report,
    /// Named macro object (step 17.1, item L4) — read-only inventory.
    Macro,
    /// VBA module (step 17.2, item L5) — extracted source, never executed.
    Module,
}

impl ObjectKind {
    /// Lower-case identifier used in deterministic test IDs (e.g. `"table"`).
    pub fn slug(self) -> &'static str {
        match self {
            ObjectKind::Table => "table",
            ObjectKind::Query => "query",
            ObjectKind::View => "view",
            ObjectKind::Form => "form",
            ObjectKind::Report => "report",
            ObjectKind::Macro => "macro",
            ObjectKind::Module => "module",
        }
    }

    /// Parses the lower-case slug the snapshot and test ids carry
    /// (`"table"`, `"query"`, …; phase 72 step 72.4).
    pub fn from_slug(slug: &str) -> Option<Self> {
        Some(match slug {
            "table" => Self::Table,
            "query" => Self::Query,
            "view" => Self::View,
            "form" => Self::Form,
            "report" => Self::Report,
            "macro" => Self::Macro,
            "module" => Self::Module,
            _ => return None,
        })
    }

    /// Whether tabs of this kind have a second (design) view (step 21.1,
    /// item U5). The read-only inventory viewers have exactly one view.
    pub fn has_design_view(self) -> bool {
        matches!(
            self,
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Form | ObjectKind::Report
        )
    }
}

/// Stable identifier of a database object (kind + name) — the anchor for
/// deterministic test IDs and command addressing (ARCHITECTURE §5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    /// Object kind (Table/Query/Form/Report).
    pub kind: ObjectKind,
    /// Case-sensitive object name as shown in the navigation pane.
    pub name: String,
}

impl ObjectId {
    /// Creates an object id from kind and name.
    pub fn new(kind: ObjectKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }

    /// State-independent, deterministic test id, e.g. `node.table.customers`.
    ///
    /// Containers compose it (e.g. `navpane.expanded.<id>`,
    /// `workspace.tab.<id>`) — they derive their IDs, never invent them.
    pub fn ui_test_id(&self) -> String {
        format!("node.{}.{}", self.kind.slug(), self.name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn object_id_renders_deterministic_node_test_id() {
        let id = ObjectId::new(ObjectKind::Table, "Customers");
        assert_eq!(id.ui_test_id(), "node.table.customers");
    }

    #[test]
    fn object_kind_slugs_are_lower_case_names() {
        assert_eq!(ObjectKind::Table.slug(), "table");
        assert_eq!(ObjectKind::Query.slug(), "query");
        assert_eq!(ObjectKind::View.slug(), "view");
        assert_eq!(ObjectKind::Form.slug(), "form");
        assert_eq!(ObjectKind::Report.slug(), "report");
        assert_eq!(ObjectKind::Macro.slug(), "macro");
        assert_eq!(ObjectKind::Module.slug(), "module");
        for kind in [
            ObjectKind::Table,
            ObjectKind::Query,
            ObjectKind::View,
            ObjectKind::Form,
            ObjectKind::Report,
            ObjectKind::Macro,
            ObjectKind::Module,
        ] {
            assert_eq!(ObjectKind::from_slug(kind.slug()), Some(kind));
        }
        assert_eq!(ObjectKind::from_slug("entity"), None);
    }

    proptest! {
        /// Any UTF-8 name survives a JSON round-trip unchanged.
        #[test]
        fn object_id_json_roundtrip_preserves_identity(name in ".*") {
            let id = ObjectId::new(ObjectKind::Query, name.as_str());
            let json = serde_json::to_string(&id).expect("serialize ObjectId");
            let back: ObjectId = serde_json::from_str(&json).expect("deserialize ObjectId");
            prop_assert_eq!(back, id);
        }
    }
}
