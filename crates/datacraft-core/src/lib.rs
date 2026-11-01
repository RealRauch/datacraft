//! DataCraft core — headless domain model, command bus and application state.
//!
//! The GUI ([`datacraft-ui`](https://docs.rs/datacraft-ui)) and future
//! MCP/JSON-RPC endpoints are thin clients of this crate: every user action
//! maps to one command, every observable outcome is queryable as serializable
//! state. See `docs/ARCHITECTURE.md` §5.

pub mod canvas;
pub mod command;
pub mod core;
pub mod database;
pub mod event;
pub mod expr;
pub mod field_expressions;
pub mod form;
pub mod forms;
pub mod grid;
pub mod ids;
pub mod journal;
pub mod macro_def;
pub mod palette;
pub mod port;
mod port_mock;
pub mod present;
pub mod qbe;
mod qbe_dispatch;
pub mod query_criteria;
pub mod query_engine;
pub mod report;
pub mod report_aggregates;
pub mod report_bands;
pub mod report_preview;
pub mod reports;
pub mod ribbon;
pub mod row_edit;
pub mod schema;
pub mod snapshot;
pub mod sql_engine;
pub mod state;
pub mod transport;
pub mod vba;
pub mod vba_analysis;
mod vba_dispatch;

pub use ids::TabId;

use serde::{Deserialize, Serialize};

/// Kind of a database object, following the Access object paradigm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Table,
    Query,
    Form,
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
            ObjectKind::Form => "form",
            ObjectKind::Report => "report",
            ObjectKind::Macro => "macro",
            ObjectKind::Module => "module",
        }
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
        assert_eq!(ObjectKind::Form.slug(), "form");
        assert_eq!(ObjectKind::Report.slug(), "report");
        assert_eq!(ObjectKind::Macro.slug(), "macro");
        assert_eq!(ObjectKind::Module.slug(), "module");
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
