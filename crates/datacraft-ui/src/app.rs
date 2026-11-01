//! Application shell (steps 5.1/6.2, items H17/H23; ARCHITECTURE §5/§8).
//!
//! Four regions: ribbon (top), navigation pane (left), tabbed document
//! workspace (center), status bar (bottom). All state lives in the headless
//! core; this crate only renders and dispatches. The snapshot is refreshed
//! after every dispatch — never per frame; row data is cached per tab
//! (the reader is a full-scan engine, so one read per tab is cheaper than
//! per-frame reads; true windowed reads arrive with usage-map discovery).

use std::collections::HashMap;

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::port::{RowRange, StoragePort};
use datacraft_core::present::status_line;
use datacraft_core::query_engine::QueryResult;
use datacraft_core::schema::FieldValue;
use datacraft_core::snapshot::UiStateSnapshot;

/// Root application state.
pub struct App {
    core: Core,
    snapshot: UiStateSnapshot,
    /// Path input buffer of the File > Open Database ribbon action.
    pub open_path: String,
    /// Selected ribbon tab id (presentation state, `&'static` from the spec).
    pub active_ribbon_tab: &'static str,
    /// Row data per table tab, loaded once on tab appearance.
    rows: HashMap<TabId, Vec<Vec<FieldValue>>>,
    /// Query results per query tab, refreshed when the QBE model changes.
    results: HashMap<TabId, QueryResult>,
    /// View mode per tab (Datasheet default; table Design is read-only).
    pub tab_mode: HashMap<TabId, TabMode>,
    /// Selected control per form-design tab.
    form_selection: HashMap<TabId, String>,
    /// Drag accumulation per (tab, control): (dx, dy), present while dragging.
    drags: HashMap<(TabId, String), (f32, f32)>,
    /// Current runtime record index per form tab.
    form_record: HashMap<TabId, usize>,
    /// Selected design-grid field index per tab.
    design_selection: HashMap<TabId, usize>,
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
    /// Success notice of the last command (status bar), cleared by
    /// failures (step 11.8).
    pub(crate) last_notice: Option<String>,
    /// Buffered cells of the datasheet's new-record row per table tab.
    new_row: HashMap<TabId, Vec<String>>,
}

/// View mode of a document tab (presentation state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabMode {
    /// The datasheet (default).
    #[default]
    Datasheet,
    /// The read-only design grid.
    Design,
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
            open_path: String::new(),
            active_ribbon_tab: "home",
            rows: HashMap::new(),
            results: HashMap::new(),
            tab_mode: HashMap::new(),
            form_selection: HashMap::new(),
            preview_page: HashMap::new(),
            previews: HashMap::new(),
            record_cursor: HashMap::new(),
            drags: HashMap::new(),
            form_record: HashMap::new(),
            design_selection: HashMap::new(),
            sql_text: "SELECT 1 AS one;".to_owned(),
            sql_error: None,
            last_error: None,
            last_notice: None,
            sidecar_path: String::new(),
            new_row: HashMap::new(),
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
            notice_for(&events)
        };
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. }))
            && let Some(db) = self.snapshot.database.as_ref()
        {
            // ADR-1: the design layer lives next to the database file.
            self.sidecar_path = datacraft_core::schema::SchemaDocument::sidecar_path(&db.path)
                .to_string_lossy()
                .into_owned();
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

    /// Row data of one table tab (cached; `None` until loaded).
    pub fn rows_for(&self, tab_id: TabId) -> Option<&Vec<Vec<FieldValue>>> {
        self.rows.get(&tab_id)
    }

    /// Query result of one query tab (cached; `None` until run).
    pub fn results_for(&self, tab_id: TabId) -> Option<&QueryResult> {
        self.results.get(&tab_id)
    }

    /// Buffered cells of a datasheet's new-record row, padded to the
    /// table's column count (step 11.7).
    pub fn new_row_cells(&self, tab_id: TabId, columns: usize) -> Vec<String> {
        let mut cells = self.new_row.get(&tab_id).cloned().unwrap_or_default();
        cells.resize(columns, String::new());
        cells
    }

    /// Stores the edited new-record cells of a datasheet tab.
    pub fn set_new_row_cells(&mut self, tab_id: TabId, cells: Vec<String>) {
        self.new_row.insert(tab_id, cells);
    }

    /// Clears the buffered new-record cells of a datasheet tab (after a
    /// successful insert).
    pub fn clear_new_row(&mut self, tab_id: TabId) {
        self.new_row.remove(&tab_id);
    }

    /// Domain definition of a table (the new-record row needs field kinds).
    pub fn table_definition(&self, table: &str) -> Option<&datacraft_core::schema::TableDef> {
        self.core.table_definition(table)
    }

    /// Rendered snapshot (read-only; the panels never mutate it).
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

    /// View mode of a tab (defaults to the datasheet).
    pub fn tab_mode(&self, tab_id: TabId) -> TabMode {
        self.tab_mode.get(&tab_id).copied().unwrap_or_default()
    }

    /// Toggles a tab between datasheet and design view.
    pub fn toggle_tab_mode(&mut self, tab_id: TabId) {
        let next = match self.tab_mode(tab_id) {
            TabMode::Datasheet => TabMode::Design,
            TabMode::Design => TabMode::Datasheet,
        };
        self.tab_mode.insert(tab_id, next);
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

    /// Loads rows for every open table tab and drops caches after
    /// database/table changes.
    fn maintain_caches(&mut self, events: &[Event]) {
        if events
            .iter()
            .any(|e| matches!(e, Event::DatabaseClosed | Event::TableDropped { .. }))
        {
            self.rows.clear();
            self.previews.clear();
            self.new_row.clear();
        }
        if events.iter().any(|e| {
            matches!(
                e,
                Event::DatabaseClosed
                    | Event::QbeCellSet { .. }
                    | Event::QueryCreated { .. }
                    | Event::QueryDropped { .. }
            )
        }) {
            self.results.clear();
        }
        // Inserted rows invalidate exactly the open datasheet tabs of their
        // table; the reload loop below repopulates the cache.
        let inserted: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                Event::RowInserted { table } => Some(table.to_lowercase()),
                _ => None,
            })
            .collect();
        if !inserted.is_empty() {
            let stale: Vec<TabId> = self
                .snapshot
                .tabs
                .iter()
                .filter(|tab| {
                    tab.object_kind == "table" && inserted.contains(&tab.object_name.to_lowercase())
                })
                .map(|tab| tab.id)
                .collect();
            for tab_id in stale {
                self.rows.remove(&tab_id);
            }
        }
        let Some(db) = &self.snapshot.database else {
            self.rows.clear();
            return;
        };
        for tab in &self.snapshot.tabs {
            if self.rows.contains_key(&tab.id) {
                continue;
            }
            // Table tabs read their own rows; form tabs read their source.
            let source_name = if tab.object_kind == "form" {
                db.forms
                    .iter()
                    .find(|f| f.name == tab.object_name)
                    .map(|f| f.source.clone())
            } else {
                Some(tab.object_name.clone())
            };
            let Some(source_name) = source_name.filter(|n| !n.is_empty()) else {
                continue;
            };
            let Some(definition) = db
                .tables
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(&source_name))
            else {
                continue;
            };
            if let Ok(rows) = self.core.read_rows(
                &definition.name,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            ) {
                self.rows.insert(tab.id, rows);
            }
        }
        // Report previews refresh with every mutation pass — design edits
        // and row changes both flow through dispatch.
        for tab in &self.snapshot.tabs {
            if tab.object_kind == "report"
                && let Ok(pages) = self.core.report_preview(&tab.object_name)
            {
                self.previews.insert(tab.id, pages);
            }
        }
        // Query results refresh whenever the QBE model changed.
        let query_tabs: Vec<(TabId, String)> = self
            .snapshot
            .tabs
            .iter()
            .filter(|tab| tab.object_kind == "query")
            .map(|tab| (tab.id, tab.object_name.clone()))
            .collect();
        for (tab_id, name) in query_tabs {
            if self.results.contains_key(&tab_id) {
                continue;
            }
            if let Ok(result) = self.core.run_query(&name) {
                self.results.insert(tab_id, result);
            }
        }
    }
}

/// Status-bar notice for successful design-document operations.
fn notice_for(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::DesignDocumentSaved { path } => Some(format!("Design saved to {}", path.display())),
        Event::DesignDocumentLoaded {
            path,
            forms,
            queries,
            reports,
        } => Some(format!(
            "Design loaded from {} ({forms} forms, {queries} queries, {reports} reports)",
            path.display()
        )),
        _ => None,
    })
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::theme::apply(ui.ctx());

        egui::Panel::top("datacraft.ribbon").show(ui, |ui| {
            crate::ribbon::ribbon_ui(self, ui);
        });

        if self.snapshot().nav_pane_visible {
            egui::Panel::left("datacraft.navpane")
                .resizable(true)
                .default_size(220.0)
                .show(ui, |ui| {
                    crate::nav_pane::nav_pane_ui(self, ui);
                });
        }

        egui::Panel::bottom("datacraft.status").show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left: database context (Access convention — the active
                // document's object replaces the generic count).
                let active = self.snapshot().active_tab.and_then(|id| {
                    self.snapshot()
                        .tabs
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| (t.object_kind.clone(), t.object_name.clone()))
                });
                ui.label(status_line(
                    self.snapshot(),
                    active.as_ref().map(|(k, n)| (k.as_str(), n.as_str())),
                ));
                if let Some(reason) = self.last_error() {
                    ui.colored_label(egui::Color32::from_rgb(0xB0, 0x2A, 0x2A), reason);
                } else if let Some(notice) = self.last_notice() {
                    ui.colored_label(crate::theme::TEAL, notice);
                }
                // Right: the record-navigation box over the active view's
                // record domain (rows / results / form record / pages).
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.record_navigation_ui(ui);
                });
            });
        });

        crate::workspace::workspace_ui(self, ui);

        crate::relationships::mapper_window(self, ui);
        crate::sql_window::sql_window(self, ui);
    }
}

impl App {
    /// The status bar's record-navigation box (step 12.4): `◀ 3 of 12 ▶`
    /// over the active tab's record domain. Paint-only presentation state —
    /// each view kind maps to its own record cursor.
    fn record_navigation_ui(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.snapshot().clone();
        let Some(active) = snapshot
            .active_tab
            .and_then(|id| snapshot.tabs.iter().find(|t| t.id == id).cloned())
        else {
            return;
        };
        let (current, len) = match active.object_kind.as_str() {
            // Forms navigate their source rows (the maintain_caches pass
            // loads them per form tab); queries walk their result set.
            "query" => (
                self.record_cursor(active.id),
                self.results_for(active.id).map_or(0, |r| r.rows.len()),
            ),
            "report" => (
                self.preview_page(active.id),
                self.preview_pages(active.id).len(),
            ),
            _ => (
                self.form_record(active.id),
                self.rows_for(active.id).map_or(0, Vec::len),
            ),
        };
        if len == 0 {
            return;
        }
        let current = current.min(len - 1);
        if ui.small_button("\u{25c0}").clicked() && current > 0 {
            self.step_record(&active, current - 1, len);
        }
        ui.label(egui::RichText::new(format!("{} of {len}", current + 1)).small());
        if ui.small_button("\u{25b6}").clicked() && current + 1 < len {
            self.step_record(&active, current + 1, len);
        }
    }

    /// Writes the stepped record position into the active view's cursor.
    fn step_record(&mut self, tab: &datacraft_core::snapshot::TabSnapshot, to: usize, len: usize) {
        match tab.object_kind.as_str() {
            "table" | "query" => self.set_record_cursor(tab.id, to, len),
            "form" => self.set_form_record(tab.id, to),
            "report" => self.set_preview_page(tab.id, to),
            _ => {}
        }
    }
}
