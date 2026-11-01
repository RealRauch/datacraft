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
    /// A row was appended to a table (step 13.2, H36).
    RowInserted {
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
    /// A control was placed on a form.
    ControlAdded {
        /// Canonical form name.
        form: String,
        /// Canonical control name.
        control: String,
    },
    /// A control was moved/resized.
    ControlRectSet {
        /// Canonical form name.
        form: String,
        /// Canonical control name.
        control: String,
    },
    /// A control was removed.
    ControlDropped {
        /// Canonical form name.
        form: String,
        /// Canonical control name.
        control: String,
    },
    /// A form's source table was assigned.
    FormSourceSet {
        /// Canonical form name.
        form: String,
        /// Canonical table name.
        table: String,
    },
    /// The design layer was written to a sidecar file.
    DesignDocumentSaved {
        /// The file written.
        path: std::path::PathBuf,
    },
    /// The design layer was loaded from a sidecar file.
    DesignDocumentLoaded {
        /// The file read.
        path: std::path::PathBuf,
        /// Number of forms loaded.
        forms: usize,
        /// Number of queries loaded.
        queries: usize,
        /// Number of reports loaded (step 10.4, H31).
        reports: usize,
    },
    /// A form was deleted.
    FormDropped {
        /// Canonical form name.
        name: String,
    },
    /// A report was created (step 10.4, H31).
    ReportCreated {
        /// Canonical report name.
        name: String,
    },
    /// A report was deleted.
    ReportDropped {
        /// Canonical report name.
        name: String,
    },
    /// A control was placed on a report.
    ReportControlAdded {
        /// Canonical report name.
        report: String,
        /// Canonical control name.
        control: String,
    },
    /// A report control was moved/resized.
    ReportControlRectSet {
        /// Canonical report name.
        report: String,
        /// Canonical control name.
        control: String,
    },
    /// A report control was removed.
    ReportControlDropped {
        /// Canonical report name.
        report: String,
        /// Canonical control name.
        control: String,
    },
    /// A report's source table was assigned.
    ReportSourceSet {
        /// Canonical report name.
        report: String,
        /// Canonical table name.
        table: String,
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
