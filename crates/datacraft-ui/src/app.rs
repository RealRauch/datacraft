//! Application shell (steps 5.1/6.2, items H17/H23; ARCHITECTURE §5/§8).
//!
//! Four regions: ribbon (top), navigation pane (left), tabbed document
//! workspace (center), status bar (bottom). All state lives in the headless
//! core; this crate only renders and dispatches. The snapshot is refreshed
//! after every dispatch — never per frame; row data is cached per tab
//! (the reader is a full-scan engine, so one read per tab is cheaper than
//! per-frame reads; true windowed reads arrive with usage-map discovery).

use std::collections::HashMap;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::port::StoragePort;
use datacraft_core::query_engine::QueryResult;
use datacraft_core::schema::FieldValue;
use datacraft_core::snapshot::UiStateSnapshot;

mod caches;
mod editors;
mod form_edit;
mod grid_state;
mod help;
mod lifecycle;
pub(crate) mod notices;

/// Root application state.
pub struct App {
    core: Core,
    snapshot: UiStateSnapshot,
    /// Path input buffer of the File > Open Database ribbon action.
    pub open_path: String,
    /// Selected ribbon tab id (presentation state, `&'static` from the spec).
    pub active_ribbon_tab: &'static str,
    /// Object kind of the active document as last seen by the ribbon
    /// (step 47.1, M58): the selection follows *kind changes* only.
    pub(crate) active_object_kind: Option<String>,
    /// Selected backstage page while the File tab is active (step 21.3).
    pub backstage_page: crate::backstage::BackstagePage,
    /// Navigation-pane group sort (step 21.5; ordering lives in core).
    pub nav_sort: datacraft_core::nav::NavSortOrder,
    /// Last viewport title sent to the window system (step 21.7, U11).
    pub(crate) window_title: String,
    /// Current ribbon key-tip phase (step 21.8, U12).
    pub keytips: Option<crate::keys::KeyTipsPhase>,
    /// Alt state of the previous frame (edge detection for key tips).
    pub(crate) alt_down: bool,
    /// Row data per table tab, loaded once on tab appearance.
    rows: HashMap<TabId, Vec<Vec<FieldValue>>>,
    /// Query results per query tab, refreshed when the QBE model changes.
    results: HashMap<TabId, QueryResult>,
    /// Cached SQL results of open view tabs (step 71.2, item M93).
    pub(crate) view_results: HashMap<TabId, datacraft_core::sql_types::SqlResult>,
    /// The open view editor draft (step 71.2, item M93).
    pub(crate) view_draft: Option<crate::views_window::ViewDraft>,
    /// The open object-rename dialog (phase 72, step 72.4; item M81).
    pub(crate) rename_draft: Option<crate::rename_dialog::RenameDraft>,
    /// Selected control per form-design tab.
    form_selection: HashMap<TabId, String>,
    /// Content drafts of the designer's control panel, keyed by tab and
    /// control (step 63.2, item H56); cleared on apply and selection.
    pub(crate) control_content_drafts: HashMap<(TabId, String), String>,
    /// Drag accumulation per (tab, control): (dx, dy), present while dragging.
    drags: HashMap<(TabId, String), (f32, f32)>,
    /// Current runtime record index per form tab.
    form_record: HashMap<TabId, usize>,
    /// Per-record edit buffer of every form tab (phase 64, M68; ADR-8).
    form_edit: form_edit::FormEditState,
    /// Selected design-grid field index per tab.
    design_selection: HashMap<TabId, usize>,
    /// Buffered new-field name per design tab (step 24.3, M9).
    design_new_field: HashMap<TabId, String>,
    filter_drafts: HashMap<(TabId, usize), String>,
    filter_dialog: HashMap<TabId, usize>,
    /// Buffered rename text per design tab (step 24.3, M9).
    design_rename: HashMap<TabId, String>,
    /// Current record cursor per table/query tab (the status bar's
    /// record-navigation box and the datasheet row selector).
    record_cursor: HashMap<TabId, usize>,
    /// Current preview page of a report tab (defaults to the first).
    preview_page: HashMap<TabId, usize>,
    /// Cached preview pages per report tab (refreshed on dispatch).
    previews: HashMap<TabId, Vec<datacraft_core::report_preview::PreviewPage>>,
    /// SQL window editor text (presentation state; the statement runs
    /// through the command bus).
    sql_text: String,
    /// Reason of the last failed SQL run (the window renders it).
    sql_error: Option<String>,
    /// Import File window buffers (presentation state; the source and the
    /// target table run through the command bus, step 58.1, item M50).
    /// Crate-visible: the accessors live in `flows.rs` (size gate).
    pub(crate) import_source: String,
    pub(crate) import_table: String,
    /// Expression window editor text (presentation state; the expression
    /// runs through the command bus, step 41.2).
    pub(crate) expression_text: String,
    /// The expression window's picked row context (presentation state;
    /// the table plus the zero-based row as typed text — step 60.1,
    /// item L27). `None` evaluates over the empty scope. Crate-visible:
    /// the accessors live in `flows.rs` (size gate).
    pub(crate) expression_context: Option<(String, String)>,
    /// Display form of the last successful expression run (the window
    /// renders it, step 41.2).
    pub(crate) expression_output: Option<String>,
    /// Reason of the last failed expression run (the window renders it).
    pub(crate) expression_error: Option<String>,
    /// Bounded notice/error history (step 68.3, item M76): the status bar
    /// shows the newest entry, the messages dialog lists the rest.
    notices: notices::NoticeLog,
    /// Whether the messages dialog is open.
    pub(crate) notices_open: bool,
    /// Whether the shortcuts help page is open (step 68.4, item M77).
    pub(crate) shortcuts_help_open: bool,
    /// Deferred busy command and its overlay state (step 68.5, item M75).
    pub(crate) busy: crate::busy::BusyState,
    /// Path buffer of the sidecar save/load ribbon actions (step 11.8).
    pub sidecar_path: String,
    /// Path buffer of the backstage New Database page (phase 29, H44).
    pub new_database_path: String,
    /// Registry name of the template selected on the New Database page.
    pub new_database_template: Option<String>,
    /// Path buffer of the backstage Save-a-copy flow (step 27.2, M14).
    pub save_as_path: String,
    /// Whether the copy is protected (the checkbox next to the path).
    pub save_as_encrypted: bool,
    /// Password buffer of the Save-a-copy flow.
    pub save_as_password: String,
    /// New-password buffer of the change-password flow.
    pub change_password: String,
    /// Buffered cells of the datasheet's new-record row per table tab.
    new_row: HashMap<TabId, Vec<String>>,
    /// The armed in-place cell editor per table tab (step 40.4, L23):
    /// (row, col) plus the editable buffer; absent while no cell is
    /// being edited.
    cell_edit: HashMap<TabId, (usize, usize, String)>,
    /// Per-tab column widths of the datasheet (presentation state; the
    /// resize math lives in core `grid`, step 21.4).
    column_widths: HashMap<TabId, Vec<f32>>,
    /// Per-tab count of frozen leading datasheet columns (presentation
    /// state; the freeze placement math lives in core `grid`, step 40.2).
    frozen_columns: HashMap<TabId, usize>,
    /// Column widths of the SQL window's result grid (step 21.4).
    sql_column_widths: Vec<f32>,
    /// Open database-password prompt (step 16.2, M5; the state and paint
    /// live in `password_prompt.rs`).
    pub(crate) password_prompt: Option<crate::password_prompt::PasswordPrompt>,
    /// VBA viewer search text (module list + source full-text filter,
    /// step 17.2).
    pub vba_search: String,
    /// Draft of the relationship-create form while the mapper pane's
    /// form is open (`None` hides it, step 41.1, item M45).
    pub(crate) relationship_draft: Option<crate::relationships::RelationshipDraft>,
    /// Cached static VBA analysis (the viewer's right pane renders it,
    /// step 41.3, item M47).
    pub(crate) vba_analysis: Option<datacraft_core::vba_analysis::VbaAnalysis>,
    /// Reason of the last failed analysis run (the viewer renders it).
    pub(crate) vba_analysis_error: Option<String>,
    /// UI-owned settings (ADR-5); persisted only when a path is attached.
    pub(crate) settings: crate::settings::Settings,
    /// Path of `ui.json`; `None` keeps settings session-only (tests).
    pub(crate) settings_path: Option<std::path::PathBuf>,
    /// Whether the restored window position was already checked (step 22.2).
    pub(crate) position_rescued: bool,
    /// Localized load failures per tab (step 39.4, item M31): rendered in
    /// place of the endless loading state, cleared on the next successful
    /// load pass.
    pub(crate) load_errors: HashMap<TabId, String>,
    /// The dialog seam: platform dialogs in the binary, scripted in the
    /// headless suite (step 38.3, item M43).
    pub(crate) dialogs: Box<dyn crate::dialogs::DialogHost>,
}

impl App {
    /// Creates the app over the given storage backend (the binary's
    /// composition root injects the real Jet4 port, BACKLOG H21).
    pub fn new(port: Box<dyn StoragePort>) -> Self {
        let core = Core::new(port);
        let snapshot = core.inspect();
        let active_ribbon_tab = Self::landing_tab(&snapshot);
        Self {
            core,
            snapshot,
            dialogs: Box::new(crate::dialogs::NativeDialogs),
            open_path: String::new(),
            active_object_kind: None,
            active_ribbon_tab,
            backstage_page: crate::backstage::BackstagePage::default(),
            nav_sort: datacraft_core::nav::NavSortOrder::default(),
            window_title: "DataCraft".to_owned(),
            keytips: None,
            alt_down: false,
            rows: HashMap::new(),
            results: HashMap::new(),
            view_results: HashMap::new(),
            view_draft: None,
            rename_draft: None,
            form_selection: HashMap::new(),
            control_content_drafts: HashMap::new(),
            preview_page: HashMap::new(),
            previews: HashMap::new(),
            record_cursor: HashMap::new(),
            load_errors: HashMap::new(),
            drags: HashMap::new(),
            form_record: HashMap::new(),
            form_edit: form_edit::FormEditState::default(),
            design_selection: HashMap::new(),
            design_new_field: HashMap::new(),
            filter_drafts: HashMap::new(),
            filter_dialog: HashMap::new(),
            design_rename: HashMap::new(),
            sql_text: "SELECT 1 AS one;".to_owned(),
            import_source: String::new(),
            import_table: String::new(),
            sql_error: None,
            expression_text: String::new(),
            expression_context: None,
            expression_output: None,
            expression_error: None,
            notices: notices::NoticeLog::default(),
            notices_open: false,
            shortcuts_help_open: false,
            busy: crate::busy::BusyState::default(),
            sidecar_path: String::new(),
            new_database_path: String::new(),
            new_database_template: None,
            save_as_path: String::new(),
            save_as_encrypted: false,
            save_as_password: String::new(),
            change_password: String::new(),
            new_row: HashMap::new(),
            cell_edit: HashMap::new(),
            column_widths: HashMap::new(),
            frozen_columns: HashMap::new(),
            sql_column_widths: Vec::new(),
            password_prompt: None,
            vba_search: String::new(),
            relationship_draft: None,
            vba_analysis: None,
            vba_analysis_error: None,
            settings: crate::settings::Settings::default(),
            settings_path: None,
            position_rescued: false,
        }
    }

    /// Applies one command and refreshes the rendered snapshot and caches.
    pub fn dispatch(&mut self, command: Command) -> Vec<Event> {
        // A form record is written before its tab (or the database)
        // closes — Access saves the record on close (ADR-8). A failed
        // write keeps the tab open with the buffer intact.
        if !self.guard_close(&command) {
            return Vec::new();
        }
        self.invalidate_combo_cache();
        let events = self.core.dispatch(command);
        self.snapshot = self.core.inspect();
        // One bounded history feeds the status bar and the messages
        // dialog (step 68.3, item M76); messages are stored semantically
        // and rendered on demand (step 70.2, item M86): the newest entry
        // wins and follows a live language switch.
        match events.iter().find_map(|event| match event {
            Event::Failed { code, reason } => Some((*code, reason.clone())),
            _ => None,
        }) {
            Some((code, reason)) => self.notices.push(
                notices::Severity::Error,
                notices::NoticeMessage::Failure { code, reason },
            ),
            None => {
                if let Some(message) = notices::notice_for(&events) {
                    self.notices.push(notices::Severity::Notice, message);
                }
            }
        }
        // Password flow (step 16.2, M5) lives in `password_prompt.rs`.
        self.handle_password_events(&events);
        // Lifecycle follow-through (step 69.3, M80; step 75.1, L41) lives
        // in `app/lifecycle.rs` (L30).
        self.follow_database_lifecycle(&events);
        self.maintain_caches(&events);
        events
    }

    /// Esc: drops the buffered new-record cells and the in-place cell
    /// editor of the active tab and closes the password prompt (step
    /// 21.8, U12; cell editor step 40.4, L23).
    pub fn cancel_edit(&mut self) {
        if let Some(tab_id) = self.snapshot.active_tab {
            self.new_row.remove(&tab_id);
            self.cell_edit.remove(&tab_id);
        }
        self.password_prompt = None;
        self.close_shortcuts_help();
    }

    /// Runs the editor's statement through the bus and caches the outcome.
    pub fn run_sql(&mut self) {
        let events = self.dispatch(Command::RunSql {
            sql: self.sql_text.clone(),
        });
        self.sql_error = events.iter().find_map(|event| match event {
            Event::Failed { code, reason } => {
                Some(crate::failure_text::failure_text(*code, reason))
            }
            _ => None,
        });
    }

    /// Result of the last successful SQL run (rendered by the window).
    pub fn sql_result(&self) -> Option<&datacraft_core::sql_engine::SqlResult> {
        self.core.sql_result()
    }

    /// Domain definition of a table (the new-record row needs field kinds).
    pub fn table_definition(&self, table: &str) -> Option<&datacraft_core::schema::TableDef> {
        self.core.table_definition(table)
    }

    /// Rendered snapshot (read-only; the panels never mutate it).
    /// A native saved query by name (step 15.3, M3; read-only).
    pub fn native_query(&self, name: &str) -> Option<&datacraft_core::port::NativeQuery> {
        self.core.native_query(name)
    }

    pub fn snapshot(&self) -> &UiStateSnapshot {
        &self.snapshot
    }

    /// Form design documents (ribbon auto-naming and the mapper).
    pub fn forms(&self) -> &[datacraft_core::form::FormDef] {
        self.snapshot
            .database
            .as_ref()
            .map(|db| &db.forms[..])
            .unwrap_or(&[])
    }

    /// Query definitions (ribbon auto-naming and the nav pane).
    pub fn queries(&self) -> &[datacraft_core::snapshot::QueryEntrySnapshot] {
        self.snapshot
            .database
            .as_ref()
            .map(|db| &db.queries[..])
            .unwrap_or(&[])
    }

    /// One extracted VBA module (the viewer's read-only data source,
    /// step 17.2).
    pub fn vba_module(&self, name: &str) -> Option<&datacraft_core::vba::VbaModule> {
        self.core.vba_module(name)
    }

    /// Runs the static VBA analysis through the bus (`AnalyzeVba`) and
    /// caches the outcome (step 41.3, item M47).
    pub fn run_vba_analysis(&mut self) {
        let events = self.dispatch(Command::AnalyzeVba);
        self.vba_analysis = None;
        self.vba_analysis_error = None;
        for event in events {
            match event {
                Event::VbaAnalyzed { analysis } => self.vba_analysis = Some(analysis),
                Event::Failed { code, reason } => {
                    self.vba_analysis_error = Some(crate::failure_text::failure_text(code, &reason))
                }
                _ => {}
            }
        }
    }

    /// Cached static VBA analysis of the open project.
    pub fn vba_analysis(&self) -> Option<&datacraft_core::vba_analysis::VbaAnalysis> {
        self.vba_analysis.as_ref()
    }

    /// Reason of the last failed analysis run (defensive: the bus may
    /// answer a failure for a stale snapshot).
    pub fn vba_analysis_error(&self) -> Option<&str> {
        self.vba_analysis_error.as_deref()
    }

    /// Current runtime record of a form tab (defaults to the first).
    pub fn form_record(&self, tab_id: TabId) -> usize {
        self.form_record.get(&tab_id).copied().unwrap_or(0)
    }

    /// Moves the runtime record cursor of a form tab.
    pub fn set_form_record(&mut self, tab_id: TabId, record: usize) {
        self.form_record.insert(tab_id, record);
    }

    /// Selected control of a form-design tab (if any).
    pub fn form_selection(&self, tab_id: TabId) -> Option<String> {
        self.form_selection.get(&tab_id).cloned()
    }

    /// Selects a form control.
    pub fn set_form_selection(&mut self, tab_id: TabId, control: String) {
        self.form_selection.insert(tab_id, control);
    }

    /// Selected control of a report-design tab (shares the per-tab control
    /// selection map with forms — tab ids never collide).
    pub fn report_selection(&self, tab_id: TabId) -> Option<String> {
        self.form_selection.get(&tab_id).cloned()
    }

    /// Selects a report control.
    pub fn set_report_selection(&mut self, tab_id: TabId, control: String) {
        self.form_selection.insert(tab_id, control);
    }

    /// Current preview page of a report tab (defaults to the first).
    pub fn preview_page(&self, tab_id: TabId) -> usize {
        self.preview_page.get(&tab_id).copied().unwrap_or(0)
    }

    /// Sets the current preview page of a report tab.
    pub fn set_preview_page(&mut self, tab_id: TabId, page: usize) {
        self.preview_page.insert(tab_id, page);
    }

    /// Current record cursor of a table/query tab (defaults to the first).
    pub fn record_cursor(&self, tab_id: TabId) -> usize {
        self.record_cursor.get(&tab_id).copied().unwrap_or(0)
    }

    /// Moves the record cursor of a table/query tab, clamped to `len`.
    pub fn set_record_cursor(&mut self, tab_id: TabId, row: usize, len: usize) {
        let clamped = len.saturating_sub(1).min(row);
        self.record_cursor.insert(tab_id, clamped);
    }

    /// Cached print-preview pages of a report tab (empty until refreshed).
    pub fn preview_pages(&self, tab_id: TabId) -> &[datacraft_core::report_preview::PreviewPage] {
        self.previews.get(&tab_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Report design documents (ribbon naming, report views).
    pub fn reports(&self) -> &[datacraft_core::report::ReportDef] {
        self.snapshot()
            .database
            .as_ref()
            .map(|db| db.reports.as_slice())
            .unwrap_or(&[])
    }

    /// Accumulates drag motion for a control (starting a gesture lazily).
    pub fn accumulate_drag(&mut self, tab_id: TabId, control: &str, dx: f32, dy: f32) {
        let entry = self
            .drags
            .entry((tab_id, control.to_owned()))
            .or_insert((0.0, 0.0));
        entry.0 += dx;
        entry.1 += dy;
    }

    /// Ends a drag and returns the accumulated delta.
    pub fn finish_drag(&mut self, tab_id: TabId, control: &str) -> Option<(f32, f32)> {
        self.drags.remove(&(tab_id, control.to_owned()))
    }

    /// Selected design-grid field index of a tab (defaults to the first).
    pub fn design_selection(&self, tab_id: TabId) -> usize {
        self.design_selection.get(&tab_id).copied().unwrap_or(0)
    }

    /// Selects a design-grid field.
    pub fn set_design_selection(&mut self, tab_id: TabId, field_index: usize) {
        self.design_selection.insert(tab_id, field_index);
    }
}
