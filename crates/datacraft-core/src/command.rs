//! The command language (step 4.1, item H13; ARCHITECTURE §5).
//!
//! Every user-visible action — GUI or agent — is exactly one command. The
//! set grows with the phases that introduce the features (QBE commands in
//! Phase 7, form commands in Phase 8); the dispatch mechanism is fixed
//! here.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::canvas::Rect;
use crate::form::ControlDef;
use crate::ids::TabId;
use crate::qbe::{QbeRow, QbeValue};
use crate::schema::TableDef;

/// Row context of `Command::RunExpression` (step 15.2, H41).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionContext {
    /// Source table name (case-insensitive).
    pub table: String,
    /// Zero-based index in the table's live-row order.
    pub row: usize,
}

/// One user-visible action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Opens the database file at `path` (closes nothing implicitly —
    /// rejected while another database is open).
    OpenDatabase {
        /// Filesystem path of the `.mdb`/`.accdb` file.
        path: PathBuf,
        /// Database password for protected files (step 16.2, M5); omitted
        /// by older clients and `None` for unprotected files.
        #[serde(default)]
        password: Option<String>,
    },
    /// Closes the open database and all of its tabs.
    CloseDatabase,
    /// Creates a brand-new, empty database file (phase 29, item H44). The
    /// file must not exist; the new database is not opened — dispatch
    /// `OpenDatabase` afterwards.
    CreateDatabase {
        /// Filesystem path of the new file.
        path: PathBuf,
        /// Target format family (`Jet4` or `Sqlite`).
        format: crate::port::DatabaseFormat,
    },
    /// Materializes a shipped template into a brand-new file (phase 29,
    /// item H44). The template's tables and rows are written through the
    /// port; the new database is not opened. Registry names are listed in
    /// the snapshot's `templates`.
    CreateDatabaseFromTemplate {
        /// Filesystem path of the new file.
        path: PathBuf,
        /// Registry name of the template (case-insensitive).
        template: String,
    },
    /// Writes a copy of the open database under `path` (step 27.2, item
    /// M14). `password` protects the copy (`None` writes an unencrypted
    /// copy); the original stays untouched and open. Gated on the driver's
    /// `passwords` capability.
    SaveDatabaseAs {
        /// Filesystem path of the copy; it must not exist yet.
        path: PathBuf,
        /// New password, or `None` for an unencrypted copy.
        #[serde(default)]
        password: Option<String>,
    },
    /// Replaces the open database's password in place (step 27.2, item
    /// M14); the old password is the one the session was opened with.
    ChangePassword {
        /// New password, or `None` to remove the protection.
        #[serde(default)]
        new_password: Option<String>,
    },
    /// Creates a table from a validated definition.
    CreateTable {
        /// Full field-level definition.
        definition: TableDef,
    },
    /// Drops a table (name matched case-insensitively); dependent tabs
    /// close first.
    DropTable {
        /// Table name as shown in the navigation pane.
        table: String,
    },
    /// Appends a field to a table (step 24.3, item M9). Existing rows get
    /// Null in the new column; the field must be nullable (or carry no
    /// engine-evaluable default).
    AddField {
        /// Target table (case-insensitive).
        table: String,
        /// Full definition of the new field.
        field: crate::schema::FieldDef,
    },
    /// Removes a field from a table (step 24.3, item M9).
    DropField {
        /// Target table (case-insensitive).
        table: String,
        /// Field name (case-insensitive).
        field: String,
    },
    /// Renames a field (step 24.3, item M9); row data is untouched.
    RenameField {
        /// Target table (case-insensitive).
        table: String,
        /// Current field name (case-insensitive).
        field: String,
        /// New field name.
        new_name: String,
    },
    /// Replaces a field's properties (step 24.3, item M9); the storage
    /// shape (type, size, kind) must not change.
    SetFieldProperties {
        /// Target table (case-insensitive).
        table: String,
        /// Field name (case-insensitive).
        field: String,
        /// The new property set.
        properties: crate::schema::FieldProperties,
    },
    /// Appends one row to a table (step 13.2, H36). Values are positional
    /// and validated against the table schema; `Null` on an AutoNumber
    /// field means "engine-assigned".
    InsertRow {
        /// Target table name (case-insensitive).
        table: String,
        /// Full positional value list (arity must match the table).
        values: Vec<crate::schema::FieldValue>,
    },
    /// Replaces one row by its zero-based live-row index (step 11.10,
    /// H40). `Null` on an AutoNumber field keeps the engine-assigned value;
    /// any other AutoNumber value must equal the current one.
    UpdateRow {
        /// Target table name (case-insensitive).
        table: String,
        /// Zero-based index in the table's live-row order.
        row: usize,
        /// Full positional value list (arity must match the table).
        values: Vec<crate::schema::FieldValue>,
    },
    /// Deletes one row by its zero-based live-row index (step 11.10, H40).
    DeleteRow {
        /// Target table name (case-insensitive).
        table: String,
        /// Zero-based index in the table's live-row order.
        row: usize,
    },
    /// Previews a journaled write batch and stores it for confirmation
    /// (step 11.11, M2). Nothing is written until `ApplyWrite`.
    PreviewWrite {
        /// Target table name (case-insensitive).
        table: String,
        /// The batch (indices address the evolving row set).
        ops: Vec<crate::journal::WriteOp>,
    },
    /// Applies the pending journal identified by `token` (step 11.11, M2).
    ApplyWrite {
        /// Token from the preview.
        token: u64,
    },
    /// Rolls the last applied journal back (inverse operations).
    RollbackWrite {
        /// Token of the applied journal.
        token: u64,
    },
    /// Discards the pending journal without writing anything.
    DiscardWrite {
        /// Token from the preview.
        token: u64,
    },
    /// Opens the object in a tab (re-activates an existing tab instead of
    /// opening a second one for the same object).
    OpenObject(ObjectId),
    /// Opens the object in the given view (step 21.5, item U9) — the
    /// navigation pane's Open/Design context menu in one command.
    OpenObjectInMode {
        /// The object to open.
        object: ObjectId,
        /// The view the tab starts (or switches to) in.
        mode: crate::state::TabMode,
    },
    /// Closes one document tab; closing the active tab activates its
    /// next-lower neighbor.
    CloseTab {
        /// The tab to close.
        tab_id: TabId,
    },
    /// Closes every tab except `tab_id` and activates the kept tab
    /// (step 21.6, item U10).
    CloseOtherTabs {
        /// The tab to keep.
        tab_id: TabId,
    },
    /// Closes every document tab (step 21.6, item U10).
    CloseAllTabs,
    /// Makes an existing tab the active one.
    SetActiveTab {
        /// The tab to activate.
        tab_id: TabId,
    },
    /// Undoes the last row write of the open database (step 28.1, item U14).
    Undo,
    /// Re-applies the last undone row write (step 28.1, item U14).
    Redo,
    /// Switches one tab between its datasheet and design view (step 21.1,
    /// item U5) — the Views split-button's single action.
    SetTabMode {
        /// The tab whose view changes.
        tab_id: TabId,
        /// The target view.
        mode: crate::state::TabMode,
    },
    /// Shows or hides the navigation pane.
    ToggleNavPane,
    /// Shows or hides the relationship mapper (Access opens it as a
    /// document; the headless core models it as a pane flag).
    ToggleRelationshipMapper,
    /// Shows or hides the SQL window (step 13.3, H37).
    ToggleSqlWindow,
    /// Runs exactly one SELECT/WITH statement over the loaded tables in a
    /// throwaway in-memory SQLite database (step 13.3, H37).
    RunSql {
        /// The statement; anything but one SELECT/WITH is rejected.
        sql: String,
    },
    /// Evaluates one expression (step 15.2, H41) — over the bus, the MCP
    /// endpoint and the CLI alike. With a row context the expression sees
    /// the table's fields; without one it evaluates over an empty scope.
    RunExpression {
        /// Optional row context (table + zero-based live-row index).
        context: Option<ExpressionContext>,
        /// The expression source.
        expression: String,
    },
    /// Creates an empty query (name unique, case-insensitive).
    CreateQuery {
        /// Query name; rejected on duplicates.
        name: String,
    },
    /// Edits one QBE grid cell; identifiers are whitelisted against the
    /// schema (LESSONS #5).
    SetQbeCell {
        /// Target query name (case-insensitive).
        query: String,
        /// Grid row.
        row: QbeRow,
        /// Grid column index.
        col: usize,
        /// New cell value.
        value: QbeValue,
    },
    /// Clears every QBE criterion cell of a query (step 11.5).
    ClearQbeCriteria {
        /// Target query name (case-insensitive).
        query: String,
    },
    /// Deletes a query (dependent tabs close first).
    DeleteQuery {
        /// Query name (case-insensitive).
        name: String,
    },
    /// Executes a query; the row count is observable, the data itself is
    /// fetched via `Core::run_query`.
    RunQuery {
        /// Query name (case-insensitive).
        query: String,
    },
    /// Creates an empty form (name unique, case-insensitive).
    CreateForm {
        /// Form name; rejected on duplicates.
        name: String,
    },
    /// Deletes a form.
    DeleteForm {
        /// Form name (case-insensitive).
        name: String,
    },
    /// Places a control on a form (rect snapped to the form grid at
    /// dispatch, bounds-checked against the canvas).
    AddFormControl {
        /// Target form name (case-insensitive).
        form: String,
        /// The control definition.
        control: ControlDef,
    },
    /// Moves/resizes a control (snapped and bounds-checked).
    SetFormControlRect {
        /// Target form name (case-insensitive).
        form: String,
        /// Control name (case-insensitive).
        control: String,
        /// New canvas rect.
        rect: Rect,
    },
    /// Removes a control.
    DeleteFormControl {
        /// Target form name (case-insensitive).
        form: String,
        /// Control name (case-insensitive).
        control: String,
    },
    /// Adds a record-source field from the designer's field list as a
    /// bound control at the next free slot (step 11.6).
    AddFormField {
        /// Target form name (case-insensitive).
        form: String,
        /// Source field name (case-insensitive).
        field: String,
    },
    /// Assigns a form's source table (whitelisted; existing bindings
    /// re-validated).
    SetFormSource {
        /// Target form name (case-insensitive).
        form: String,
        /// Source table name.
        table: String,
    },
    /// Creates an empty report with print-oriented page defaults
    /// (step 10.4, item H31).
    CreateReport {
        /// Report name; rejected on duplicates.
        name: String,
    },
    /// Deletes a report.
    DeleteReport {
        /// Report name (case-insensitive).
        name: String,
    },
    /// Places a control on a report (page-bounds-checked, snapped).
    AddReportControl {
        /// Target report name (case-insensitive).
        report: String,
        /// The control definition.
        control: ControlDef,
    },
    /// Moves/resizes a report control (snapped, page-bounds-checked).
    SetReportControlRect {
        /// Target report name (case-insensitive).
        report: String,
        /// Control name (case-insensitive).
        control: String,
        /// New page rect.
        rect: Rect,
    },
    /// Removes a report control.
    DeleteReportControl {
        /// Target report name (case-insensitive).
        report: String,
        /// Control name (case-insensitive).
        control: String,
    },
    /// Adds a record-source field from the designer's field list as a
    /// bound control at the next free page slot (step 11.6).
    AddReportField {
        /// Target report name (case-insensitive).
        report: String,
        /// Source field name (case-insensitive).
        field: String,
    },
    /// Assigns a report's source table (whitelisted; bindings re-validated).
    SetReportSource {
        /// Target report name (case-insensitive).
        report: String,
        /// Source table name.
        table: String,
    },
    /// Sets or clears a report's grouping field (step 15.2, H41): the
    /// field must exist in the source; `None` clears the grouping.
    SetReportGrouping {
        /// Target report name (case-insensitive).
        report: String,
        /// The grouping field, or `None` to ungroup.
        group_by: Option<String>,
    },
    /// Loads one VBA module's extracted source (step 17.2, L5). Read-only:
    /// the source is returned in `Event::VbaModuleLoaded`, never executed.
    LoadVbaModule {
        /// Module name (case-insensitive).
        module: String,
    },
    /// Runs the headless static VBA analysis (step 17.3, L5): call graph,
    /// embedded SQL, table/query references and unused modules.
    AnalyzeVba,
    /// Writes the design layer (forms + queries + reports) to a
    /// `.datacraft` sidecar file (ADR-1).
    SaveDesignDocument {
        /// Target file path.
        path: std::path::PathBuf,
    },
    /// Loads forms + queries from a sidecar file (version gate enforced;
    /// the database name must match the open database).
    LoadDesignDocument {
        /// Source file path.
        path: std::path::PathBuf,
    },
}
