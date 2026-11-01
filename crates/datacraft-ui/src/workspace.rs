//! Tabbed document workspace (step 5.4, item H20).
//!
//! Tab strip and central area render purely from the snapshot: tabs carry
//! their convention test ids (`workspace.tab.<kind>.<name>`) as push_id
//! salts, clicking activates (`SetActiveTab`), the ✕ closes
//! (`CloseTab`), and the central area shows the phase placeholder for the
//! active tab. Intents are collected during paint and applied after —
//! no per-frame snapshot clones, no hidden state.

use eframe::egui;

use crate::app::TabMode;
use datacraft_core::command::Command;
use datacraft_core::ids::TabId;

use crate::app::App;

/// Renders the tab strip and the central placeholder area.
pub fn workspace_ui(app: &mut App, ui: &mut egui::Ui) {
    let empty = app.snapshot().tabs.is_empty();
    let no_database = app.snapshot().database.is_none();
    if empty {
        ui.centered_and_justified(|ui| {
            if no_database {
                ui.weak("Open a database from the File ribbon tab.");
            } else {
                ui.weak("No open documents");
            }
        });
        return;
    }

    // Tab strip: collect intents, then apply after the borrow ends.
    let mut intent: Option<WorkspaceIntent> = None;
    egui::Panel::top("datacraft.tabstrip")
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let snapshot = app.snapshot();
                let mut toggled: Option<TabId> = None;
                for tab in &snapshot.tabs {
                    ui.push_id(&tab.ui_test_id, |ui| {
                        let is_active = Some(tab.id) == snapshot.active_tab;
                        if ui.selectable_label(is_active, &tab.object_name).clicked() {
                            intent = Some(WorkspaceIntent::Activate(tab.id));
                        }
                        if ui
                            .small_button(match app.tab_mode(tab.id) {
                                TabMode::Datasheet => "Design",
                                TabMode::Design => "Data",
                            })
                            .clicked()
                        {
                            toggled = Some(tab.id);
                        }
                        if ui.small_button("✕").clicked() {
                            intent = Some(WorkspaceIntent::Close(tab.id));
                        }
                    });
                    ui.separator();
                }
                if let Some(tab_id) = toggled {
                    app.toggle_tab_mode(tab_id);
                }
            });
        });
    if let Some(applied) = intent {
        apply(app, applied);
    }

    // Central area renders the post-dispatch snapshot (immediate feedback).
    egui::CentralPanel::default().show(ui, |ui| {
        let snapshot = app.snapshot().clone();
        let active = snapshot
            .active_tab
            .and_then(|id| snapshot.tabs.iter().find(|tab| tab.id == id).cloned());
        match active {
            Some(tab) if tab.object_kind == "form" => {
                crate::form_design::form_design_ui(app, &tab, ui)
            }
            Some(tab) if tab.object_kind == "query" => match app.tab_mode(tab.id) {
                TabMode::Datasheet => crate::datasheet::query_results_ui(app, &tab, ui),
                TabMode::Design => crate::qbe::qbe_ui(app, &tab, ui),
            },
            Some(tab) => match app.tab_mode(tab.id) {
                TabMode::Datasheet => crate::datasheet::datasheet_ui(app, &tab, ui),
                TabMode::Design => crate::design::design_ui(app, &tab, ui),
            },
            None => {
                ui.centered_and_justified(|ui| ui.weak("Select a tab"));
            }
        }
    });
}

enum WorkspaceIntent {
    Activate(TabId),
    Close(TabId),
}

fn apply(app: &mut App, intent: WorkspaceIntent) {
    match intent {
        WorkspaceIntent::Activate(tab_id) => app.dispatch(Command::SetActiveTab { tab_id }),
        WorkspaceIntent::Close(tab_id) => app.dispatch(Command::CloseTab { tab_id }),
    }
}
