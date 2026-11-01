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

    /// Rendered snapshot (read-only; the panels never mutate it).
    pub fn snapshot(&self) -> &UiStateSnapshot {
        &self.snapshot
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
        let Some(db) = &self.snapshot.database else {
            self.rows.clear();
            return;
        };
        for tab in &self.snapshot.tabs {
            if self.rows.contains_key(&tab.id) {
                continue;
            }
            let Some(definition) = db
                .tables
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(&tab.object_name))
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
    }
}
