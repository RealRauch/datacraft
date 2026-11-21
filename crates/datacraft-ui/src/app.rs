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
use datacraft_core::snapshot::UiStateSnapshot;

mod caches;
pub(crate) mod dialog_flow;
mod edit_batch;
mod editors;
pub(crate) mod filter_builder;
mod form_edit;
mod grid_state;
mod help;
mod lifecycle;
pub(crate) mod nav_history;
pub(crate) mod notices;
mod state;

pub use nav_history::ReferencedCount;
pub use state::App;

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
            dialogs: Box::new(crate::dialogs::NativeDialogs::default()),
            clipboard: Box::new(crate::clipboard::SystemClipboard),
            pending_dialog: None,
            open_path: String::new(),
            active_object_kind: None,
            active_ribbon_tab,
            backstage_page: crate::backstage::BackstagePage::default(),
            nav_sort: datacraft_core::nav::NavSortOrder::default(),
            nav_search: String::new(),
            nav_selection: Vec::new(),
            nav_anchor: None,
            group_draft: None,
            render_mode: crate::render_mode::RenderMode::default(),
            last_frame: None,
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
            selections: HashMap::new(),
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
            new_database: crate::backstage::NewDatabaseDraft::default(),
            save_as_path: String::new(),
            save_as_encrypted: false,
            save_as_password: String::new(),
            change_password: String::new(),
            new_row: HashMap::new(),
            cell_edit: HashMap::new(),
            column_widths: HashMap::new(),
            frozen_columns: HashMap::new(),
            sql_column_widths: Vec::new(),
            sql_selection: datacraft_core::selection::Selection::collapse(0, 0),
            value_viewer: HashMap::new(),
            value_editor: HashMap::new(),
            password_prompt: None,
            vba_search: String::new(),
            relationship_draft: None,
            vba_analysis: None,
            vba_analysis_error: None,
            settings: crate::settings::Settings::default(),
            settings_path: None,
            position_rescued: false,
            edit_buffers: HashMap::new(),
            cursor_slots: HashMap::new(),
            submission: None,
            fk_histories: HashMap::new(),
            count_results: HashMap::new(),
            count_pending: Vec::new(),
            count_menu_tab: None,
            filter_builder_drafts: HashMap::new(),
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
        // The pending-edit batch follows the journal events (step 85.2,
        // item H61); its state lives in `app/edit_batch.rs`.
        self.maintain_submissions(&events);
        events
    }

    /// Esc: drops the buffered new-record cells and the in-place cell
    /// editor of the active tab and closes the password prompt (step
    /// 21.8, U12; cell editor step 40.4, L23). With staged datasheet
    /// edits it discards the active record's staged changes (step 85.2,
    /// item H61) — never while a cell editor is armed, which Esc cancels
    /// first.
    pub fn cancel_edit(&mut self) {
        if let Some(tab_id) = self.snapshot.active_tab {
            self.new_row.remove(&tab_id);
            self.cell_edit.remove(&tab_id);
            self.discard_staged_record(tab_id);
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
