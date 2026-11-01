//! Application shell (step 5.1, item H17; ARCHITECTURE §5/§8).
//!
//! Four regions: ribbon (top), navigation pane (left), tabbed document
//! workspace (center), status bar (bottom). All state lives in the headless
//! core; this crate only renders and dispatches. The snapshot is refreshed
//! after every dispatch — never per frame (no per-frame allocations on the
//! hot path).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::present::status_line;
use datacraft_core::snapshot::UiStateSnapshot;

/// Root application state.
pub struct App {
    core: Core,
    snapshot: UiStateSnapshot,
}

impl App {
    /// Creates the app over a fresh in-memory backend (real Jet4 wiring
    /// arrives with the storage port adapter, BACKLOG H16).
    pub fn new() -> Self {
        let core = Core::new(Box::new(MockPort::new()));
        let snapshot = core.inspect();
        Self { core, snapshot }
    }

    /// Applies one command and refreshes the rendered snapshot.
    pub fn dispatch(&mut self, command: Command) {
        self.core.dispatch(command);
        self.snapshot = self.core.inspect();
    }

    /// Rendered snapshot (read-only; the panels never mutate it).
    pub fn snapshot(&self) -> &UiStateSnapshot {
        &self.snapshot
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("datacraft.ribbon").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("DataCraft");
                ui.separator();
                if ui.button("Toggle Navigation").clicked() {
                    self.dispatch(Command::ToggleNavPane);
                }
                ui.label("Ribbon — step 5.2");
            });
        });

        if self.snapshot().nav_pane_visible {
            egui::Panel::left("datacraft.navpane").show(ui, |ui| {
                ui.heading("Navigation");
                if let Some(db) = &self.snapshot().database {
                    for table in &db.tables {
                        ui.push_id(&table.ui_test_id, |ui| {
                            ui.label(&table.name);
                        });
                    }
                } else {
                    ui.label("No database open");
                }
            });
        }

        egui::Panel::bottom("datacraft.status").show(ui, |ui| {
            ui.label(status_line(self.snapshot()));
        });

        egui::CentralPanel::default().show(ui, |ui| {
            match &self.snapshot().database {
                Some(_) => ui.label("Tabbed workspace — step 5.4"),
                None => ui.label("Open a database from the File ribbon tab."),
            };
        });
    }
}
