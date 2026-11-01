//! Tabbed document workspace (step 5.4, item H20).
//!
//! Tab strip and central area render purely from the snapshot: tabs carry
//! their convention test ids (`workspace.tab.<kind>.<name>`) as push_id
//! salts, clicking activates (`SetActiveTab`), the ✕ closes
//! (`CloseTab`), and the central area shows the phase placeholder for the
//! active tab. Intents are collected during paint and applied after —
//! no per-frame snapshot clones, no hidden state.

use eframe::egui;

use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::present::tab_placeholder;

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
                for tab in &snapshot.tabs {
                    ui.push_id(&tab.ui_test_id, |ui| {
                        let is_active = Some(tab.id) == snapshot.active_tab;
                        if ui.selectable_label(is_active, &tab.object_name).clicked() {
                            intent = Some(WorkspaceIntent::Activate(tab.id));
                        }
                        if ui.small_button("✕").clicked() {
                            intent = Some(WorkspaceIntent::Close(tab.id));
                        }
                    });
                    ui.separator();
                }
            });
        });
    if let Some(applied) = intent {
        apply(app, applied);
    }

    // Central area renders the post-dispatch snapshot (immediate feedback).
    egui::CentralPanel::default().show(ui, |ui| {
        let snapshot = app.snapshot();
        let active = snapshot
            .active_tab
            .and_then(|id| snapshot.tabs.iter().find(|tab| tab.id == id));
        ui.centered_and_justified(|ui| match active {
            Some(tab) => ui.label(tab_placeholder(kind_of(&tab.object_kind), &tab.object_name)),
            None => ui.weak("Select a tab"),
        });
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

/// Maps a snapshot kind slug back onto the domain kind (tabs hold slugs for
/// serializable snapshots).
fn kind_of(slug: &str) -> ObjectKind {
    match slug {
        "query" => ObjectKind::Query,
        "form" => ObjectKind::Form,
        "report" => ObjectKind::Report,
        _ => ObjectKind::Table,
    }
}
