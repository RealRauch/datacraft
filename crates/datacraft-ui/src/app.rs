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
mod grid_state;

/// Root application state.
pub struct App {
    core: Core,
    snapshot: UiStateSnapshot,
    /// Path input buffer of the File > Open Database ribbon action.
    pub open_path: String,
    /// Selected ribbon tab id (presentation state, `&'static` from the spec).
    pub active_ribbon_tab: &'static str,
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
    /// Selected control per form-design tab.
    form_selection: HashMap<TabId, String>,
    /// Drag accumulation per (tab, control): (dx, dy), present while dragging.
    drags: HashMap<(TabId, String), (f32, f32)>,
    /// Current runtime record index per form tab.
    form_record: HashMap<TabId, usize>,
    /// Selected design-grid field index per tab.
    design_selection: HashMap<TabId, usize>,
    /// Buffered new-field name per design tab (step 24.3, M9).
    design_new_field: HashMap<TabId, String>,
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
    /// Reason of the last failed command (status bar).
    pub(crate) last_error: Option<String>,
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
    /// Success notice of the last command (status bar), cleared by
    /// failures (step 11.8).
    pub(crate) last_notice: Option<String>,
    /// Buffered cells of the datasheet's new-record row per table tab.
    new_row: HashMap<TabId, Vec<String>>,
    /// Per-tab column widths of the datasheet (presentation state; the
    /// resize math lives in core `grid`, step 21.4).
    column_widths: HashMap<TabId, Vec<f32>>,
    /// Column widths of the SQL window's result grid (step 21.4).
    sql_column_widths: Vec<f32>,
    /// Open database-password prompt (step 16.2, M5; the state and paint
    /// live in `password_prompt.rs`).
    pub(crate) password_prompt: Option<crate::password_prompt::PasswordPrompt>,
    /// VBA viewer search text (module list + source full-text filter,
    /// step 17.2).
    pub vba_search: String,
    /// UI-owned settings (ADR-5); persisted only when a path is attached.
    pub(crate) settings: crate::settings::Settings,
    /// Path of `ui.json`; `None` keeps settings session-only (tests).
    pub(crate) settings_path: Option<std::path::PathBuf>,
    /// Whether the restored window position was already checked (step 22.2).
    pub(crate) position_rescued: bool,
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
        Self {
            core,
            snapshot,
            dialogs: Box::new(crate::dialogs::NativeDialogs),
            open_path: String::new(),
            active_ribbon_tab: "home",
            backstage_page: crate::backstage::BackstagePage::default(),
            nav_sort: datacraft_core::nav::NavSortOrder::default(),
            window_title: "DataCraft".to_owned(),
            keytips: None,
            alt_down: false,
            rows: HashMap::new(),
            results: HashMap::new(),
            form_selection: HashMap::new(),
            preview_page: HashMap::new(),
            previews: HashMap::new(),
            record_cursor: HashMap::new(),
            drags: HashMap::new(),
            form_record: HashMap::new(),
            design_selection: HashMap::new(),
            design_new_field: HashMap::new(),
            design_rename: HashMap::new(),
            sql_text: "SELECT 1 AS one;".to_owned(),
            sql_error: None,
            last_error: None,
            last_notice: None,
            sidecar_path: String::new(),
            new_database_path: String::new(),
            new_database_template: None,
            save_as_path: String::new(),
            save_as_encrypted: false,
            save_as_password: String::new(),
            change_password: String::new(),
            new_row: HashMap::new(),
            column_widths: HashMap::new(),
            sql_column_widths: Vec::new(),
            password_prompt: None,
            vba_search: String::new(),
            settings: crate::settings::Settings::default(),
            settings_path: None,
            position_rescued: false,
        }
    }

    /// Applies one command and refreshes the rendered snapshot and caches.
    pub fn dispatch(&mut self, command: Command) -> Vec<Event> {
        let events = self.core.dispatch(command);
        self.snapshot = self.core.inspect();
        self.last_error = events.iter().find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        });
        self.last_notice = if self.last_error.is_some() {
            None
        } else {
            crate::flows::notice_for(&events)
        };
        // Password flow (step 16.2, M5) lives in `password_prompt.rs`.
        self.handle_password_events(&events);
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. }))
            && let Some(db) = self.snapshot.database.as_ref()
        {
            // ADR-1: the design layer lives next to the database file.
            let path = db.path.clone();
            self.sidecar_path = datacraft_core::schema::SchemaDocument::sidecar_path(&path)
                .to_string_lossy()
                .into_owned();
            // Item U15: a successful open feeds the bounded recent list.
            self.record_recent_database(&path);
        }
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseClosed))
        {
            self.sidecar_path.clear();
        }
        self.maintain_caches(&events);
        events
    }

    /// Esc: drops the buffered new-record cells of the active tab and
    /// closes the password prompt (step 21.8, U12).
    pub fn cancel_edit(&mut self) {
        if let Some(tab_id) = self.snapshot.active_tab {
            self.new_row.remove(&tab_id);
        }
        self.password_prompt = None;
    }

    /// SQL window editor text.
    pub fn sql_text(&self) -> &str {
        &self.sql_text
    }

    /// Replaces the SQL editor text.
    pub fn set_sql_text(&mut self, text: impl Into<String>) {
        self.sql_text = text.into();
    }

    /// Runs the editor's statement through the bus and caches the outcome.
    pub fn run_sql(&mut self) {
        let events = self.dispatch(Command::RunSql {
            sql: self.sql_text.clone(),
        });
        self.sql_error = events.iter().find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        });
    }

    /// Result of the last successful SQL run (rendered by the window).
    pub fn sql_result(&self) -> Option<&datacraft_core::sql_engine::SqlResult> {
        self.core.sql_result()
    }

    /// Precise reason of the last failed SQL run.
    pub fn sql_error(&self) -> Option<&str> {
        self.sql_error.as_deref()
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
