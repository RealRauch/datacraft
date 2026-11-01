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
use crate::appearance::Appearance;
use crate::i18n::{t, tr};
use crate::icons;
use crate::settings::LanguageSetting;
use crate::theme::GEOMETRY;
use crate::theme::fonts;
use crate::theme::tokens;

/// Renders the ribbon in Access structure (step 12.2): a tab strip row on
/// top, the active tab's command groups below — each group carries its
/// caption at the bottom and groups are separated by hairlines. Pure
/// paint: intents are collected during paint and dispatched once.
pub fn ribbon_ui(app: &mut App, ui: &mut egui::Ui) {
    let spec = ribbon_spec();
    let tokens = tokens(ui.visuals());

    // Tab strip row: the active tab reads as a teal selection.
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        for tab in &spec.tabs {
            let selected = app.active_ribbon_tab == tab.id;
            let label = egui::RichText::new(t(tab.label)).strong();
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
                if group.id == "design" {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.sidecar_path)
                            .desired_width(160.0)
                            .hint_text(t("path to .datacraft")),
                    );
                }
                ui.horizontal(|ui| {
                    for action in &group.actions {
                        let id = format!("ribbon.{}.{}.{}", tab.id, group.id, action.id);
                        ui.push_id(id, |ui| match &action.state {
                            ActionState::Enabled => {
                                if action.id == "table" {
                                    if ribbon_button(ui, t(action.label), action.id) {
                                        app.create_table_flow();
                                    }
                                } else if action.id == "open_database" {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.open_path)
                                                .desired_width(180.0)
                                                .hint_text(t("path to .mdb")),
                                        );
                                        if ribbon_button(ui, t(action.label), action.id) {
                                            intent = Some(Command::OpenDatabase {
                                                path: app.open_path.clone().into(),
                                                password: None,
                                            });
                                        }
                                    });
                                } else if ribbon_button(ui, t(action.label), action.id)
                                    && let Some(command) = command_for(action.id, app)
                                {
                                    intent = Some(command);
                                }
                            }
                            ActionState::DisabledIn { phase } => {
                                ribbon_button_disabled(ui, t(action.label), action.id)
                                    .on_hover_text(tr("arrives in {phase}", &[("phase", phase)]));
                            }
                        });
                    }
                });
                ui.add_space(1.0);
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(t(group.label))
                                .size(fonts::CAPTION.size)
                                .extra_letter_spacing(fonts::CAPTION.tracking_points())
                                .color(tokens.text_muted),
                        );
                    },
                );
            });
            ui.separator();
        }
        if tab.id == "file" {
            language_group(ui, app);
            appearance_group(ui, app);
        }
    });
    if let Some(command) = intent {
        app.dispatch(command);
    }
}

/// Language switcher (step 19.2, item I5): the File tab owns the UI-only
/// language choice; selecting applies live and persists to `ui.json`.
fn language_group(ui: &mut egui::Ui, app: &mut App) {
    let tokens = tokens(ui.visuals());
    ui.vertical(|ui| {
        ui.set_min_width(120.0);
        ui.add_space(2.0);
        let current = app.language_setting();
        egui::ComboBox::from_id_salt("ribbon.file.language")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for choice in [
                    LanguageSetting::Auto,
                    LanguageSetting::German,
                    LanguageSetting::English,
                ] {
                    if ui
                        .selectable_label(current == choice, choice.label())
                        .clicked()
                    {
                        app.set_language_setting(choice);
                    }
                }
            });
        ui.add_space(1.0);
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(t("Language"))
                        .size(fonts::CAPTION.size)
                        .extra_letter_spacing(fonts::CAPTION.tracking_points())
                        .color(tokens.text_muted),
                );
            },
        );
    });
    ui.separator();
}

/// Appearance switcher (step 20.4, item U4): the File tab owns the
/// Light/Dark/System choice; selecting applies live and persists to
/// `ui.json`.
fn appearance_group(ui: &mut egui::Ui, app: &mut App) {
    let tokens = tokens(ui.visuals());
    ui.vertical(|ui| {
        ui.set_min_width(120.0);
        ui.add_space(2.0);
        let current = app.appearance_setting();
        egui::ComboBox::from_id_salt("ribbon.file.appearance")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for choice in [Appearance::System, Appearance::Light, Appearance::Dark] {
                    if ui
                        .selectable_label(current == choice, choice.label())
                        .clicked()
                    {
                        app.set_appearance_setting(choice);
                    }
                }
            });
        ui.add_space(1.0);
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(t("Appearance"))
                        .size(fonts::CAPTION.size)
                        .extra_letter_spacing(fonts::CAPTION.tracking_points())
                        .color(tokens.text_muted),
                );
            },
        );
    });
    ui.separator();
}

/// An Access-sized command button (icon above the label).
fn ribbon_button(ui: &mut egui::Ui, label: &str, icon: &str) -> bool {
    ribbon_button_response(ui, label, icon, true).clicked()
}

/// The disabled twin: painted muted, hovers with the phase hint.
fn ribbon_button_disabled(ui: &mut egui::Ui, label: &str, icon: &str) -> egui::Response {
    ribbon_button_response(ui, label, icon, false)
}

/// Custom-painted command button. A real egui button cannot host the vector
/// icon, so the button is allocated and painted here; the AccessKit label
/// keeps label-based addressing intact.
fn ribbon_button_response(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    enabled: bool,
) -> egui::Response {
    let tokens = tokens(ui.visuals());
    let font = egui::FontId::new(fonts::CONTROL.size, egui::FontFamily::Proportional);
    let measure = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), tokens.text);
    let width = (measure.size().x + 18.0).max(52.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 44.0), sense);

    let visuals = if enabled {
        *ui.style().interact(&response)
    } else {
        ui.style().visuals.widgets.noninteractive
    };
    ui.painter().rect(
        rect,
        GEOMETRY.widget_corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );

    let ink = if enabled {
        visuals.fg_stroke.color
    } else {
        tokens.text_disabled
    };
    let accent = if enabled {
        tokens.accent
    } else {
        tokens.text_disabled
    };
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 13.0),
        egui::vec2(icons::ICON_SIZE, icons::ICON_SIZE),
    );
    icons::paint(ui.painter(), icon_rect, icon, ink, accent);

    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, ink);
    ui.painter().galley(
        egui::pos2(rect.center().x, rect.bottom() - 12.0) - galley.size() / 2.0,
        galley,
        ink,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    response
}

/// Maps a static ribbon action onto its command. `None` for the
/// input-bound `open_database` (its command is built in the paint code)
/// and for unknown ids — the seam contract test pins the set.
pub fn command_for(action_id: &str, app: &App) -> Option<Command> {
    match action_id {
        "close_database" => Some(Command::CloseDatabase),
        "toggle_nav_pane" => Some(Command::ToggleNavPane),
        "relationships" => Some(Command::ToggleRelationshipMapper),
        "save_design" => Some(Command::SaveDesignDocument {
            path: app.sidecar_path.clone().into(),
        }),
        "load_design" => Some(Command::LoadDesignDocument {
            path: app.sidecar_path.clone().into(),
        }),
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
                            if action.id == "table" {
                                continue;
                            }
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
