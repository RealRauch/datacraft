//! Observable outcomes of dispatch (step 4.1, item H13; ARCHITECTURE §5).
//!
//! The UI re-renders from state, agents assert on events. Every event is
//! serializable so headless transports (Phase 9) forward it verbatim.

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::ids::TabId;
use crate::port::DatabaseFormat;

/// One observable outcome of a dispatched command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// A database was opened successfully.
    DatabaseOpened {
        /// Database name (file stem).
        name: String,
        /// Binary format family reported by the backend.
        format: DatabaseFormat,
    },
    /// The open database was closed (all tabs closed first).
    DatabaseClosed,
    /// A table was created.
    TableCreated {
        /// Canonical table name.
        table: String,
    },
    /// A table was dropped (dependent tabs closed first).
    TableDropped {
        /// Canonical table name.
        table: String,
    },
    /// A new tab was opened (followed by `ActiveTabChanged`).
    TabOpened {
        /// Assigned tab id.
        tab_id: TabId,
        /// The object shown in the tab.
        object: ObjectId,
    },
    /// A tab was closed.
    TabClosed {
        /// The closed tab id.
        tab_id: TabId,
    },
    /// The active tab changed (including to `None` when none is left).
    ActiveTabChanged {
        /// The newly active tab, if any.
        tab_id: Option<TabId>,
    },
    /// The navigation pane visibility changed.
    NavPaneToggled {
        /// Visibility after the toggle.
        visible: bool,
    },
    /// A query was created.
    QueryCreated {
        /// Canonical query name.
        name: String,
    },
    /// A QBE cell was set.
    QbeCellSet {
        /// Canonical query name.
        query: String,
    },
    /// A form was created.
    FormCreated {
        /// Canonical form name.
        name: String,
    },
    /// A form was deleted.
    FormDropped {
        /// Canonical form name.
        name: String,
    },
    /// A query executed successfully.
    QueryRan {
        /// Canonical query name.
        query: String,
        /// Number of result rows.
        rows: usize,
    },
    /// A query was deleted (dependent tabs closed first).
    QueryDropped {
        /// Canonical query name.
        name: String,
    },
    /// The relationship mapper visibility changed.
    RelationshipMapperToggled {
        /// Openness after the toggle.
        open: bool,
    },
    /// A command could not be executed; the reason is precise and
    /// user-renderable (LESSONS #4 — no silent swallowing).
    Failed {
        /// Why the command failed.
        reason: String,
    },
}
