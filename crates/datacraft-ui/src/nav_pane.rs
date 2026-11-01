//! Navigation pane rendering (step 5.3, item H19).
//!
//! Fully snapshot-driven: tables come from `UiStateSnapshot` with their
//! convention test ids (`navpane.<expanded|collapsed>.node.<kind>.<name>`)
//! used as egui id salts; a click dispatches `OpenObject`. No hidden state.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;

use crate::app::App;

/// Renders the navigation pane contents (the panel itself is shown by the
/// shell only while `nav_pane_visible`).
pub fn nav_pane_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("All Access Objects").strong());

    let snapshot = app.snapshot();
    let Some(db) = &snapshot.database else {
        ui.label("No database open");
        return;
    };
    if db.tables.is_empty() {
        ui.label("No objects");
        return;
    }
    let active_object = snapshot
        .active_tab
        .and_then(|active| active_object_name(&snapshot.tabs, active));

    let mut intent: Option<String> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for table in &db.tables {
            ui.push_id(&table.ui_test_id, |ui| {
                let is_active = active_object
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&table.name));
                if ui.selectable_label(is_active, &table.name).clicked() {
                    intent = Some(table.name.clone());
                }
            });
        }
    });
    if let Some(name) = intent {
        app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            datacraft_core::ObjectKind::Table,
            name,
        )));
    }
}

fn active_object_name(
    tabs: &[datacraft_core::snapshot::TabSnapshot],
    active: TabId,
) -> Option<String> {
    tabs.iter()
        .find(|tab| tab.id == active)
        .map(|tab| tab.object_name.clone())
}
