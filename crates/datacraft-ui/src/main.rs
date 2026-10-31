//! DataCraft desktop shell (eframe/egui).
//!
//! Phase 1 skeleton: proves the GUI stack builds and opens a window. The full
//! layout (ribbon, navigation pane, tabbed workspace) is built in Phase 5 on
//! top of the headless core (see `docs/ARCHITECTURE.md` §3/§5).
//!
//! Coverage policy: this crate is rendering glue only — logic lives in
//! `datacraft-core`; the crate is excluded from the coverage gate
//! (exception registry in `docs/TESTING.md`).

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("DataCraft", options, Box::new(|_cc| Ok(Box::new(App))))
}

/// Root application state. Phase 5 replaces this stub with the MDI layout
/// driven by `datacraft-core` commands.
#[derive(Default)]
struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("DataCraft");
        ui.label("Phase 1 skeleton — UI layout arrives in Phase 5.");
    }
}
