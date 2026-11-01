//! Ribbon rendering (step 5.2, item H18; fix F1).
//!
//! Paint code only: renders `datacraft_core::ribbon::ribbon_spec()` and
//! maps the (spec-locked) enabled actions onto commands; the open action
//! binds the path buffer that lives on `App`. The mapping is a total,
//! tested function — a spec/mapping disagreement surfaces as a contract
//! failure, not a runtime panic (LESSONS #2: seam tested from both sides).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{ActionState, ribbon_spec};

use crate::app::App;
use crate::theme::INK_SOFT;

/// Renders the ribbon in Access structure (step 12.2): a tab strip row on
/// top, the active tab's command groups below — each group carries its
/// caption at the bottom and groups are separated by hairlines. Pure
/// paint: intents are collected during paint and dispatched once.
pub fn ribbon_ui(app: &mut App, ui: &mut egui::Ui) {
    let spec = ribbon_spec();

    // Tab strip row: the active tab reads as a teal selection.
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        for tab in &spec.tabs {
            let selected = app.active_ribbon_tab == tab.id;
            let label = egui::RichText::new(tab.label).strong();
            if ui.selectable_label(selected, label).clicked() {
                app.active_ribbon_tab = tab.id;
            }
        }
    });
    ui.add(egui::Separator::default().horizontal());

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
        ui.add_space(4.0);
        for group in &tab.groups {
            // Group: actions flow horizontally, the caption sits centred
            // at the bottom (the Access ribbon convention).
            ui.vertical(|ui| {
                ui.set_min_width(120.0);
                ui.add_space(2.0);
                ui.horizontal(|ui| {
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
                                        if ribbon_button(ui, action.label) {
                                            intent = Some(Command::OpenDatabase {
                                                path: app.open_path.clone().into(),
                                            });
                                        }
                                    });
                                } else if ribbon_button(ui, action.label)
                                    && let Some(command) = command_for(action.id, app)
                                {
                                    intent = Some(command);
                                }
                            }
                            ActionState::DisabledIn { phase } => {
                                ui.add_enabled(false, ribbon_button_shape(action.label))
                                    .on_disabled_hover_text(format!("arrives in {phase}"));
                            }
                        });
                    }
                });
                ui.add_space(1.0);
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        ui.label(egui::RichText::new(group.label).small().color(INK_SOFT));
                    },
                );
            });
            ui.separator();
        }
    });
    if let Some(command) = intent {
        app.dispatch(command);
    }
}

/// An Access-sized command button (comfortable hit target).
fn ribbon_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(ribbon_button_shape(label)).clicked()
}

fn ribbon_button_shape(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.to_owned()).strong())
        .min_size(egui::vec2(0.0, 28.0))
}

/// Maps a static ribbon action onto its command. `None` for the
/// input-bound `open_database` (its command is built in the paint code)
/// and for unknown ids — the seam contract test pins the set.
pub fn command_for(action_id: &str, app: &App) -> Option<Command> {
    match action_id {
        "close_database" => Some(Command::CloseDatabase),
        "toggle_nav_pane" => Some(Command::ToggleNavPane),
        "relationships" => Some(Command::ToggleRelationshipMapper),
        "query_sql" => Some(Command::ToggleSqlWindow),
        "query_design" => Some(Command::CreateQuery {
            name: next_query_name(app),
        }),
        "form_design" => Some(Command::CreateForm {
            name: next_form_name(app),
        }),
        "report_design" => Some(Command::CreateReport {
            name: next_report_name(app),
        }),
        _ => None,
    }
}

/// First free "FormN" name (case-insensitive against the snapshot).
fn next_form_name(app: &App) -> String {
    next_name(
        app.forms().iter().map(|f| f.name.to_lowercase()).collect(),
        "Form",
    )
}

/// First free "ReportN" name (case-insensitive against the snapshot).
fn next_report_name(app: &App) -> String {
    next_name(
        app.reports()
            .iter()
            .map(|r| r.name.to_lowercase())
            .collect(),
        "Report",
    )
}

/// First free "QueryN" name (case-insensitive against the snapshot).
fn next_query_name(app: &App) -> String {
    next_name(
        app.queries()
            .iter()
            .map(|q| q.name.to_lowercase())
            .collect(),
        "Query",
    )
}

fn next_name(used: Vec<String>, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !used.contains(&name.to_lowercase()))
        .unwrap_or_else(|| prefix.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use datacraft_core::port::MockPort;

    /// Seam contract (LESSONS #2): every enabled spec action except the
    /// input-bound open_database maps to a command; nothing else does.
    #[test]
    fn every_enabled_action_maps_to_a_command() {
        let app = App::new(Box::new(MockPort::new()));
        for tab in &ribbon_spec().tabs {
            for group in &tab.groups {
                for action in &group.actions {
                    let mapped = command_for(action.id, &app);
                    match &action.state {
                        ActionState::Enabled if action.id != "open_database" => {
                            assert!(
                                mapped.is_some(),
                                "enabled action {} has no mapping",
                                action.id
                            );
                        }
                        _ => assert!(
                            mapped.is_none(),
                            "{} must not map a static command",
                            action.id
                        ),
                    }
                }
            }
        }
    }
}
