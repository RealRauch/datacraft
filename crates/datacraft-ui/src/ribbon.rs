//! Ribbon rendering (step 5.2, item H18).
//!
//! Paint code only: renders `datacraft_core::ribbon::ribbon_spec()` and
//! maps the (spec-locked) enabled actions onto commands; the open action
//! binds the path buffer that lives on `App`. Widgets derive their ids
//! from the convention strings — never invent them.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{ActionState, ribbon_spec};

use crate::app::App;

/// Renders the ribbon (tab strip + active tab's groups).
pub fn ribbon_ui(app: &mut App, ui: &mut egui::Ui) {
    let spec = ribbon_spec();

    ui.horizontal(|ui| {
        for tab in &spec.tabs {
            let selected = app.active_ribbon_tab == tab.id;
            if ui.selectable_label(selected, tab.label).clicked() {
                app.active_ribbon_tab = tab.id;
            }
        }
    });
    ui.separator();

    let Some(tab) = spec
        .tabs
        .iter()
        .find(|t| t.id == app.active_ribbon_tab)
        .cloned()
    else {
        return;
    };
    let mut intent: Option<Command> = None;
    ui.horizontal(|ui| {
        for group in &tab.groups {
            ui.vertical(|ui| {
                ui.set_min_width(120.0);
                for action in &group.actions {
                    let id = format!("ribbon.{}.{}.{}", tab.id, group.id, action.id);
                    ui.push_id(id, |ui| match &action.state {
                        ActionState::Enabled => {
                            if action.id == "open_database" {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.open_path)
                                            .desired_width(180.0)
                                            .hint_text("path to .mdb"),
                                    );
                                    if ui.button(action.label).clicked() {
                                        intent = Some(Command::OpenDatabase {
                                            path: app.open_path.clone().into(),
                                        });
                                    }
                                });
                            } else if ui.button(action.label).clicked() {
                                intent = Some(match action.id {
                                    "close_database" => Command::CloseDatabase,
                                    "toggle_nav_pane" => Command::ToggleNavPane,
                                    "relationships" => Command::ToggleRelationshipMapper,
                                    other => unreachable!(
                                        "spec and mapping disagree on {other:?} (contract-tested)"
                                    ),
                                });
                            }
                        }
                        ActionState::DisabledIn { phase } => {
                            ui.add_enabled(false, egui::Button::new(action.label))
                                .on_disabled_hover_text(format!("arrives in {phase}"));
                        }
                    });
                }
                ui.label(egui::RichText::new(group.label).small());
            });
            ui.separator();
        }
    });
    if let Some(command) = intent {
        app.dispatch(command);
    }
}
