//! Ribbon rendering (step 5.2, item H18; fix F1; QAT in step 21.2, U6).
//!
//! Paint code only: renders `datacraft_core::ribbon::ribbon_spec()` and
//! maps the (spec-locked) enabled actions onto commands; the open action
//! binds the path buffer that lives on `App`. The mapping is a total,
//! tested function — a spec/mapping disagreement surfaces as a contract
//! failure, not a runtime panic (LESSONS #2: seam tested from both sides).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{ActionState, rendered_tabs};

use crate::availability;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::icons;
use crate::theme::GEOMETRY;
use crate::theme::fonts;
use crate::theme::tokens;

mod qat;
mod views;

/// Renders the Quick Access strip and the ribbon in Access structure
/// (step 12.2): a compact icon row on top, then a tab strip row and the
/// active tab's command groups — each group carries its caption at the
/// bottom and groups are separated by hairlines. Pure paint: intents are
/// collected during paint and dispatched once.
pub fn ribbon_ui(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    let mut intent: Option<Command> = qat::quick_access_ui(app, ui);
    // Contextual tabs follow the active object kind (step 21.1, U5); a
    // contextual tab that no longer applies falls back to Home.
    let active_kind = views::active_object_kind(app);
    let tabs = rendered_tabs(active_kind.as_deref());
    if !tabs.iter().any(|tab| tab.id == app.active_ribbon_tab) {
        app.active_ribbon_tab = "home";
    }

    // Tab strip row: the active tab reads as a teal selection. In the key
    // tip Tabs phase each tab shows its spec-derived tip (step 21.8, U12).
    let tab_tips = if app.keytips == Some(crate::keys::KeyTipsPhase::Tabs) {
        let ids: Vec<&str> = tabs.iter().map(|tab| tab.id).collect();
        datacraft_core::keys::key_tips(&ids)
    } else {
        Vec::new()
    };
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        for (index, tab) in tabs.iter().enumerate() {
            let selected = app.active_ribbon_tab == tab.id;
            let label_text = match tab_tips.get(index) {
                Some(tip) => format!("[{tip}] {}", t(tab.label)),
                None => t(tab.label).to_owned(),
            };
            let label = egui::RichText::new(label_text).strong();
            if ui.selectable_label(selected, label).clicked() {
                app.active_ribbon_tab = tab.id;
            }
        }
    });
    ui.add(egui::Separator::default().horizontal());

    let Some(tab) = tabs
        .iter()
        .find(|tab| tab.id == app.active_ribbon_tab)
        .map(|tab| (*tab).clone())
    else {
        return;
    };
    // Commands phase: tips on every action button, in spec order.
    let command_tips = if app.keytips == Some(crate::keys::KeyTipsPhase::Commands) {
        let ids: Vec<&str> = tab
            .groups
            .iter()
            .flat_map(|group| group.actions.iter())
            .map(|action| action.id)
            .collect();
        datacraft_core::keys::key_tips(&ids)
    } else {
        Vec::new()
    };
    let mut tip_index = 0usize;
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
                        let tip = command_tips.get(tip_index).map(String::as_str);
                        tip_index += 1;
                        let id = format!("ribbon.{}.{}.{}", tab.id, group.id, action.id);
                        ui.push_id(id, |ui| match &action.state {
                            ActionState::Enabled
                                if availability::restricted(app.snapshot(), action.id)
                                    .is_some() =>
                            {
                                // Capability-disabled for the open source
                                // (ADR-7): same muted rendering as the
                                // planned actions, reason instead of phase.
                                let reason = availability::restricted(app.snapshot(), action.id)
                                    .expect("checked above");
                                ribbon_button_disabled(ui, t(action.label), action.id, tip)
                                    .on_hover_text(availability::reason_text(reason));
                            }
                            ActionState::Enabled => {
                                if action.id == "table" {
                                    if ribbon_button(ui, t(action.label), action.id, tip) {
                                        app.create_table_flow();
                                    }
                                } else if action.id == "open_database" {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.open_path)
                                                .desired_width(180.0)
                                                .hint_text(t("path to .mdb")),
                                        );
                                        if ribbon_button(ui, t(action.label), action.id, tip) {
                                            intent = Some(Command::OpenDatabase {
                                                path: app.open_path.clone().into(),
                                                password: None,
                                            });
                                        }
                                    });
                                } else if action.id == "view_mode" {
                                    if let Some(command) = views::view_mode_button(ui, app, tip) {
                                        intent = Some(command);
                                    }
                                } else if ribbon_button(ui, t(action.label), action.id, tip)
                                    && let Some(command) = command_for(action.id, app)
                                {
                                    intent = Some(command);
                                }
                            }
                            ActionState::DisabledIn { phase } => {
                                ribbon_button_disabled(ui, t(action.label), action.id, tip)
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
    });
    if let Some(command) = intent {
        app.dispatch(command);
    }
}

/// An Access-sized command button (icon above the label).
fn ribbon_button(ui: &mut egui::Ui, label: &str, icon: &str, tip: Option<&str>) -> bool {
    ribbon_button_response(ui, label, icon, true, tip).clicked()
}

/// The disabled twin: painted muted, hovers with the phase hint.
fn ribbon_button_disabled(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    tip: Option<&str>,
) -> egui::Response {
    ribbon_button_response(ui, label, icon, false, tip)
}

/// Custom-painted command button. A real egui button cannot host the vector
/// icon, so the button is allocated and painted here; the AccessKit label
/// keeps label-based addressing intact.
fn ribbon_button_response(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    enabled: bool,
    tip: Option<&str>,
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
    if let Some(tip) = tip {
        // Key-tip badge (step 21.8, U12): painted, never part of the
        // AccessKit label or the translated caption.
        let badge = egui::Rect::from_min_size(
            rect.right_top() + egui::vec2(-16.0, 2.0),
            egui::vec2(14.0, 12.0),
        );
        ui.painter().rect_filled(badge, 2.0, tokens.accent);
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            tip,
            egui::FontId::new(9.0, egui::FontFamily::Proportional),
            tokens.on_accent,
        );
    }
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
        "undo" => Some(Command::Undo),
        "redo" => Some(Command::Redo),
        "relationships" => Some(Command::ToggleRelationshipMapper),
        "save_design" => Some(Command::SaveDesignDocument {
            path: app.sidecar_path.clone().into(),
        }),
        "load_design" => Some(Command::LoadDesignDocument {
            path: app.sidecar_path.clone().into(),
        }),
        "query_sql" => Some(Command::ToggleSqlWindow),
        "query_expression" => Some(Command::ToggleExpressionWindow),
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
    use datacraft_core::ribbon::ribbon_spec;

    /// Seam contract (LESSONS #2): every enabled spec action except the
    /// input-bound `open_database`, the state-bound `view_mode` and the
    /// UI-flow `table` maps to a command; nothing else does.
    #[test]
    fn every_enabled_action_maps_to_a_command() {
        let app = App::new(Box::new(MockPort::new()));
        for tab in rendered_tabs(Some("table")) {
            for group in &tab.groups {
                for action in &group.actions {
                    let mapped = command_for(action.id, &app);
                    match &action.state {
                        ActionState::Enabled
                            if !matches!(action.id, "open_database" | "view_mode" | "table") =>
                        {
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

    /// The QAT reuses the ribbon action ids, so both surfaces resolve
    /// through the same mapping; disabled entries map nothing (step 21.2).
    #[test]
    fn quick_access_entries_use_the_shared_command_mapping() {
        let app = App::new(Box::new(MockPort::new()));
        for entry in &ribbon_spec().quick_access {
            let mapped = command_for(entry.id, &app);
            match &entry.state {
                ActionState::Enabled => {
                    assert!(mapped.is_some(), "QAT {} has no mapping", entry.id);
                }
                ActionState::DisabledIn { .. } => {
                    assert!(mapped.is_none(), "QAT {} must stay disabled", entry.id);
                }
            }
        }
        assert!(matches!(
            command_for("save_design", &app),
            Some(Command::SaveDesignDocument { .. })
        ));
    }
}
