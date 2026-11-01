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
    /// View mode per tab (Datasheet default; Design is read-only in Phase 6).
    pub tab_mode: HashMap<TabId, TabMode>,
    /// Selected control per form-design tab.
    form_selection: HashMap<TabId, String>,
    /// Drag accumulation per (tab, control): (dx, dy), present while dragging.
    drags: HashMap<(TabId, String), (f32, f32)>,
    /// Current runtime record index per form tab.
    form_record: HashMap<TabId, usize>,
    /// Selected design-grid field index per tab.
    design_selection: HashMap<TabId, usize>,
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
            drags: HashMap::new(),
            form_record: HashMap::new(),
            design_selection: HashMap::new(),
        }
    }

    /// Applies one command and refreshes the rendered snapshot and caches.
    pub fn dispatch(&mut self, command: Command) {
        let events = self.core.dispatch(command);
        self.snapshot = self.core.inspect();
        self.maintain_caches(&events);
    }

    /// Row data of one table tab (cached; `None` until loaded).
    pub fn rows_for(&self, tab_id: TabId) -> Option<&Vec<Vec<FieldValue>>> {
        self.rows.get(&tab_id)
    }

    /// Query result of one query tab (cached; `None` until run).
    pub fn results_for(&self, tab_id: TabId) -> Option<&QueryResult> {
        self.results.get(&tab_id)
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

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
            ui.label(status_line(self.snapshot()));
        });

        crate::workspace::workspace_ui(self, ui);

        crate::relationships::mapper_window(self, ui);
    }
}
