//! The command language (step 4.1, item H13; ARCHITECTURE §5): every
//! user-visible action — GUI or agent — is exactly one command. The enum
//! is `#[non_exhaustive]` (M22, ADR-7): adding a command is not a breaking
//! change for clients compiled against a published engine — out-of-crate
//! consumers match with a wildcard arm; wire (serde) consumers are
//! unaffected.

use crate::ObjectId;
use crate::canvas::Rect;
use crate::form::ControlDef;
use crate::ids::TabId;
use crate::qbe::{QbeRow, QbeValue, SortOrder};
use crate::schema::TableDef;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod cells;
mod contexts;
pub use cells::CellEdit;
pub use contexts::ExpressionContext;
mod source;
pub use source::{SourceSpec, effective_path};

/// One user-visible action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Command {
    /// Opens the database file at `path`; rejected while another database is open.
    OpenDatabase {
        /// Filesystem path of the `.mdb`/`.accdb` file.
        path: PathBuf,
        /// Database password (step 16.2, M5); absent/`None` = unprotected.
        #[serde(default)]
        password: Option<String>,
        /// Source override (step 65.1, L8; ADR-7): `None` opens `path`
        /// byte-identically; `Some` resolves through [`SourceSpec`]
        /// (`file://` like its path, others answer the no-server boundary).
        #[serde(default)]
        source: Option<SourceSpec>,
    },
    /// Closes the open database and all of its tabs.
    CloseDatabase,
    /// Compacts the open database in place (95.2, M126): atomic verified swap.
    CompactDatabase,
    /// Creates a brand-new, empty database file (H44); the file must not exist.
    CreateDatabase {
        /// Filesystem path of the new file.
        path: PathBuf,
        /// Target format family (`Jet4` or `Sqlite`).
        format: crate::port::DatabaseFormat,
    },
    /// Materializes a shipped template into a new file (29, H44); not opened.
    CreateDatabaseFromTemplate {
        /// Filesystem path of the new file.
        path: PathBuf,
        /// Registry name of the template (case-insensitive).
        template: String,
    },
    /// Writes a copy under `path` (step 27.2, M14), original untouched.
    SaveDatabaseAs {
        /// Filesystem path of the copy; it must not exist yet.
        path: PathBuf,
        /// New password, or `None` for an unencrypted copy.
        #[serde(default)]
        password: Option<String>,
    },
    /// Replaces the password in place (27.2, M14); the open session's one.
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
    /// Imports an external CSV as a new table (58.1, M50); source untouched.
    ImportFile {
        /// The CSV file to read (header row + records).
        source: std::path::PathBuf,
        /// Target table name.
        table: String,
    },
    /// Drops a table (case-insensitive); dependent tabs close first.
    DropTable {
        /// Table name as shown in the navigation pane.
        table: String,
    },
    /// Appends a field (24.3, M9); existing rows get Null, nullable required.
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
    /// Replaces a field's properties (24.3, M9); the storage shape stays.
    SetFieldProperties {
        /// Target table (case-insensitive).
        table: String,
        /// Field name (case-insensitive).
        field: String,
        /// The new property set.
        properties: crate::schema::FieldProperties,
    },
    /// Appends one row (13.2, H36); `Null` on AutoNumber = engine-assigned.
    InsertRow {
        /// Target table name (case-insensitive).
        table: String,
        /// Full positional value list (arity must match the table).
        values: Vec<crate::schema::FieldValue>,
    },
    /// Replaces one row by index (11.10, H40); `Null` keeps the AutoNumber.
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
    /// Replaces individual cells across rows (86.3, M101): one history entry.
    UpdateCells {
        /// Target table name (case-insensitive).
        table: String,
        /// The cell edits, grouped per row by the handler.
        edits: Vec<crate::command::CellEdit>,
    },
    /// Previews a journaled write batch (11.11, M2); headless-only (L26).
    PreviewWrite {
        /// Target table name (case-insensitive).
        table: String,
        /// The batch (indices address the evolving row set).
        ops: Vec<crate::journal::WriteOp>,
    },
    /// Applies the pending journal (step 11.11, M2). Headless-only (L26).
    ApplyWrite {
        /// Token from the preview.
        token: u64,
    },
    /// Rolls the last applied journal back. Headless-only (L26).
    RollbackWrite {
        /// Token of the applied journal.
        token: u64,
    },
    /// Discards the pending journal without writing. Headless-only (L26).
    DiscardWrite {
        /// Token from the preview.
        token: u64,
    },
    /// Renames one table, query, form or report (72.1, M81); references follow.
    RenameObject {
        /// The object to rename (kind + current name).
        object: ObjectId,
        /// The new name, validated against the kind's namespace.
        new_name: String,
    },
    /// Opens the object in a tab (an already-open object re-activates).
    OpenObject(ObjectId),
    /// Opens the object in the given view (21.5, U9) — the nav pane's menu.
    OpenObjectInMode {
        /// The object to open.
        object: ObjectId,
        /// The view the tab starts (or switches to) in.
        mode: crate::state::TabMode,
    },
    /// Closes one document tab; the active tab activates its neighbor.
    CloseTab {
        /// The tab to close.
        tab_id: TabId,
    },
    /// Closes every tab except the kept one (step 21.6, U10).
    CloseOtherTabs {
        /// The tab to keep.
        tab_id: TabId,
    },
    /// Closes every tab strictly left of the anchor; it survives (88.3).
    CloseTabsToLeft {
        /// The tab that anchors the action and survives.
        tab_id: TabId,
    },
    /// Closes every tab strictly right of the anchor; it survives (88.3).
    CloseTabsToRight {
        /// The tab that anchors the action and survives.
        tab_id: TabId,
    },
    /// Closes every document tab (step 21.6, item U10).
    CloseAllTabs,
    /// Makes an existing tab the active one.
    SetActiveTab {
        /// The tab to activate.
        tab_id: TabId,
    },
    /// Undoes the last row write of the open database (step 28.1, U14).
    Undo,
    /// Re-applies the last undone row write (step 28.1, U14).
    Redo,
    /// Switches one tab between datasheet and design view (21.1, U5).
    SetTabMode {
        /// The tab whose view changes.
        tab_id: TabId,
        /// The target view.
        mode: crate::state::TabMode,
    },
    /// Shows or hides the navigation pane.
    ToggleNavPane,
    /// Shows or hides the relationship mapper pane (step 7.1).
    ToggleRelationshipMapper,
    /// Creates a referential relationship (M16); duplicates are refused.
    CreateRelationship {
        /// The relationship to create.
        relationship: crate::port::Relationship,
    },
    /// Deletes one referential relationship (step 32.3, item M16).
    DeleteRelationship {
        /// The relationship to delete.
        relationship: crate::port::Relationship,
    },
    /// Shows or hides the SQL window (step 13.3, H37).
    ToggleSqlWindow,
    /// Opens or closes the Import File window (step 58.1, item M50).
    ToggleImportWindow,
    /// Parses an external file for the Import File window (58.1, M50).
    PreviewImportFile {
        /// The CSV file to read.
        source: std::path::PathBuf,
    },
    /// Shows or hides the expression window (step 41.2, item M46).
    ToggleExpressionWindow,
    /// Runs one SELECT/WITH in throwaway SQLite (13.3, H37).
    RunSql {
        /// The statement; anything but one SELECT/WITH is rejected.
        sql: String,
    },
    /// Evaluates one expression (H41); the optional context supplies fields.
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
    /// Sidecar view design command (step 71.1, M93): create, set SQL, delete.
    View(crate::view::ViewCommand),
    /// Edits one QBE grid cell (identifiers are schema-whitelisted, LESSONS #5).
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
    /// Sorts a datasheet view (44.5, M54); the field is whitelisted (LESSONS #5).
    SetTableSort {
        /// Target table name (case-insensitive).
        table: String,
        /// Sort field name.
        field: String,
        /// Sort direction; `None` clears the sort.
        order: SortOrder,
    },
    /// Filters a datasheet column by criteria (step 44.5, M54); filters AND.
    SetTableFilter {
        /// Target table name (case-insensitive).
        table: String,
        /// Filtered field name.
        field: String,
        /// Criteria text, engine dialect; `None` clears the field.
        criteria: Option<String>,
    },
    /// Clears every column filter of a datasheet (step 44.5, M54).
    ClearTableFilters {
        /// Target table name (case-insensitive).
        table: String,
    },
    /// Lists one column's distinct rendered values (phase 101, M132).
    ListColumnValues {
        /// Target table name (case-insensitive).
        table: String,
        /// The column to list.
        field: String,
    },
    /// Filters a column by a value from its list (phase 101, M132);
    /// `None` picks the blank entry.
    FilterByValue {
        /// Target table name (case-insensitive).
        table: String,
        /// The filtered column.
        field: String,
        /// The rendered value, or `None` for blanks.
        value: Option<String>,
    },
    /// Deletes a query (dependent tabs close first).
    DeleteQuery {
        /// Query name (case-insensitive).
        name: String,
    },
    /// Executes a query; the data itself is fetched via `Core::run_query`.
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
    /// Places a control on a form (rect snapped and bounds-checked).
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
    /// Sets the content property a placed control carries (step 63.1,
    /// H56): label, caption, bound field or embedded form; empty clears.
    SetFormControlContent {
        /// Target form name (case-insensitive).
        form: String,
        /// Control name (case-insensitive).
        control: String,
        /// The content value.
        value: String,
    },
    /// Sets the kind-specific design data of a placed form control (ADR-8).
    SetFormControlData(crate::form::SetControlData),
    /// Applies or clears the form's own WHERE filter (phase 64, M71; ADR-8).
    SetFormFilter(crate::form::FormFilter),
    /// Removes a control.
    DeleteFormControl {
        /// Target form name (case-insensitive).
        form: String,
        /// Control name (case-insensitive).
        control: String,
    },
    /// Adds a record-source field as a bound control at the next free slot (11.6).
    AddFormField {
        /// Target form name (case-insensitive).
        form: String,
        /// Source field name (case-insensitive).
        field: String,
    },
    /// Assigns a form's source table (whitelisted; bindings re-validated).
    SetFormSource {
        /// Target form name (case-insensitive).
        form: String,
        /// Source table name.
        table: String,
    },
    /// Creates an empty report with print-oriented page defaults (H31).
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
    /// Sets the content property a placed report control carries (step
    /// 63.1, H56): label, caption, bound field or `=expression`; empty clears.
    SetReportControlContent {
        /// Target report name (case-insensitive).
        report: String,
        /// Control name (case-insensitive).
        control: String,
        /// The content value.
        value: String,
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
    /// Sets or clears a report's grouping field (step 15.2, H41); it must
    /// exist in the source.
    SetReportGrouping {
        /// Target report name (case-insensitive).
        report: String,
        /// The grouping field, or `None` to ungroup.
        group_by: Option<String>,
    },
    /// Loads one VBA module's extracted source (read-only, never executed).
    LoadVbaModule {
        /// Module name (case-insensitive).
        module: String,
    },
    /// Runs the static VBA analysis (step 17.3, L5): call graph, SQL,
    /// table/query references and unused modules.
    AnalyzeVba,
    /// Writes the design layer (forms + queries + reports) to a `.datacraft` sidecar (ADR-1).
    SaveDesignDocument {
        /// Target file path.
        path: std::path::PathBuf,
    },
    /// Loads a sidecar (version gate enforced; the database name must match).
    LoadDesignDocument {
        /// Source file path.
        path: std::path::PathBuf,
    },
    /// Exports one report's print preview to a new PDF file (phase 97, M128).
    ExportReportPdf {
        /// Report name (case-insensitive).
        report: String,
        /// Target file; it must not exist yet.
        path: PathBuf,
    },
    /// Exports one table to a new CSV or JSON file (phase 99, M130).
    ExportTable {
        /// Table name (case-insensitive).
        table: String,
        /// Output format of the document.
        format: crate::export::ExportFormat,
        /// Target file; it must not exist yet.
        path: PathBuf,
    },
}
