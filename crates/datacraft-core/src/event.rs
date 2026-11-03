//! Observable outcomes of dispatch (step 4.1, item H13; ARCHITECTURE §5).
//!
//! The UI re-renders from state, agents assert on events. Every event is
//! serializable so headless transports (Phase 9) forward it verbatim.

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::ids::TabId;
use crate::port::{DatabaseFormat, Relationship};

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
    /// A copy of the open database was written under a new path
    /// (step 27.2, item M14). The original stays untouched and open.
    DatabaseSavedAs {
        /// Filesystem path of the copy.
        path: std::path::PathBuf,
        /// Whether the copy is password-protected.
        encrypted: bool,
    },
    /// The open database's password was replaced in place (step 27.2,
    /// item M14).
    PasswordChanged {
        /// Whether the database is now password-protected.
        encrypted: bool,
    },
    /// A new database file was created (phase 29, item H44); it is not
    /// opened — an `OpenDatabase` command follows.
    DatabaseCreated {
        /// Filesystem path of the new file.
        path: std::path::PathBuf,
        /// Format family of the created file.
        format: DatabaseFormat,
    },
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
    /// A table's schema changed (step 24.3, item M9).
    TableAltered {
        /// Canonical table name.
        table: String,
        /// The affected field (canonical name after the change).
        field: String,
    },
    /// A row was appended to a table (step 13.2, H36).
    RowInserted {
        /// Canonical table name.
        table: String,
    },
    /// A row was replaced (step 11.10, H40).
    RowUpdated {
        /// Canonical table name.
        table: String,
        /// Zero-based index in the live-row order.
        row: usize,
    },
    /// An expression was evaluated over the bus (step 15.2, H41); the
    /// typed value is queryable via `Core::expression_result`.
    ExpressionEvaluated {
        /// The result rendered through the shared presentation layer.
        result: String,
    },
    /// A report's grouping was set or cleared (step 15.2, H41).
    ReportGroupingSet {
        /// Canonical report name.
        report: String,
        /// The grouping field, or `None` when ungrouped.
        group_by: Option<String>,
    },
    /// The last row write was undone (step 28.1, item U14).
    Undone {
        /// Canonical table name.
        table: String,
    },
    /// The last undone row write was re-applied (step 28.1, item U14).
    Redone {
        /// Canonical table name.
        table: String,
    },
    /// A row was deleted (step 11.10, H40).
    RowDeleted {
        /// Canonical table name.
        table: String,
        /// Zero-based index in the live-row order at delete time.
        row: usize,
    },
    /// A write batch was previewed and awaits confirmation (step 11.11).
    WritePreviewed {
        /// Canonical table name.
        table: String,
        /// Confirmation token for `ApplyWrite`.
        token: u64,
        /// The diff, one entry per operation.
        changes: Vec<crate::journal::RowChange>,
    },
    /// A previewed write batch was applied (step 11.11).
    WriteApplied {
        /// Canonical table name.
        table: String,
        /// Token of the applied journal.
        token: u64,
        /// Rows appended.
        inserted: usize,
        /// Rows replaced.
        updated: usize,
        /// Rows removed.
        deleted: usize,
        /// Inverse operations in reverse application order.
        rollback: Vec<crate::journal::WriteOp>,
    },
    /// The last applied journal was rolled back (step 11.11).
    WriteRolledBack {
        /// Canonical table name.
        table: String,
        /// Token of the rolled-back journal.
        token: u64,
    },
    /// A previewed write batch was discarded without writing.
    WriteDiscarded {
        /// Token of the discarded journal.
        token: u64,
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
    /// A tab's view mode changed (step 21.1, item U5).
    TabModeSet {
        /// The tab whose view changed.
        tab_id: TabId,
        /// The view after the change.
        mode: crate::state::TabMode,
    },
    /// The navigation pane visibility changed.
    NavPaneToggled {
        /// Visibility after the toggle.
        visible: bool,
    },
    /// A query's criteria were cleared (step 11.5).
    QbeCriteriaCleared {
        /// Canonical query name.
        query: String,
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
    RelationshipCreated {
        /// The created relationship.
        relationship: Relationship,
    },
    RelationshipDeleted {
        /// The deleted relationship.
        relationship: Relationship,
    },
    RelationshipMapperToggled {
        /// Openness after the toggle.
        open: bool,
    },
    /// The SQL window visibility changed (step 13.3, H37).
    SqlWindowToggled {
        /// Openness after the toggle.
        open: bool,
    },
    /// A SQL statement ran successfully (step 13.3, H37); the result data
    /// is fetched via `Core::run_sql`/the cached result.
    SqlRan {
        /// Number of result rows.
        rows: usize,
    },
    /// One VBA module's source was loaded over the bus (step 17.2, L5).
    VbaModuleLoaded {
        /// The module including its decoded source.
        module: crate::vba::VbaModule,
    },
    /// The headless static VBA analysis completed (step 17.3, L5).
    VbaAnalyzed {
        /// Call graph, SQL literals, references and unused modules.
        analysis: crate::vba_analysis::VbaAnalysis,
    },
    /// A command could not be executed; the reason is precise and
    /// user-renderable (LESSONS #4 — no silent swallowing).
    Failed {
        /// Why the command failed.
        reason: String,
    },
    /// The database is password-protected and the open command carried no
    /// password (step 16.2, M5) — the UI raises its prompt.
    PasswordRequired {
        /// The file that needs a password.
        path: std::path::PathBuf,
    },
    /// The supplied password does not match the database (step 16.2, M5).
    PasswordRejected {
        /// The file whose password was rejected.
        path: std::path::PathBuf,
    },
}
