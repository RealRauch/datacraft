//! Ribbon rendering (step 5.2, item H18; fix F1; QAT in step 21.2, U6).
//!
//! Paint code only: renders `datacraft_core::ribbon::ribbon_spec()` and
//! maps the (spec-locked) enabled actions onto commands; the open action
//! binds the path buffer that lives on `App`. The mapping is a total,
//! tested function — a spec/mapping disagreement surfaces as a contract
//! failure, not a runtime panic (LESSONS #2: seam tested from both sides).

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{RibbonGroup, rendered_tabs};

use crate::layout_probe;

use crate::app::App;
use crate::i18n::t;
use crate::icons;
use crate::theme::GEOMETRY;
use crate::theme::fonts;
use crate::theme::tokens;

mod actions;
mod qat;
mod views;

/// The ribbon selection follows the active document's object kind
/// (step 47.1, M58 — Access convention): activating a table, query,
/// form or report surfaces its contextual tools tab. Two guards keep
/// it honest: the backstage is never yanked, and a same-kind
/// activation (switching between two tables) changes nothing — the
/// user keeps, say, Database Tools while browsing tables.
fn follow_active_object(app: &mut App) {
    let kind = app
        .snapshot()
        .active_tab
        .and_then(|id| app.snapshot().tabs.iter().find(|tab| tab.id == id))
        .map(|tab| tab.object_kind.clone());
    if kind == app.active_object_kind {
        return;
    }
    app.active_object_kind = kind.clone();
    if app.active_ribbon_tab == "file" {
        return;
    }
    if let Some(kind) = kind
        && let Some(tab_id) = datacraft_core::ribbon::contextual_tab_id(&kind)
    {
        app.active_ribbon_tab = tab_id;
    }
}

/// Renders the Quick Access strip and the ribbon in Access structure
/// (step 12.2): a compact icon row on top, then a tab strip row and the
/// active tab's command groups — each group carries its caption at the
/// bottom and groups are separated by hairlines. Pure paint: intents are
/// collected during paint and dispatched once.
pub fn ribbon_ui(app: &mut App, ui: &mut egui::Ui) {
    follow_active_object(app);
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
        // The shortcut page's visible affordance (step 69.4, item M82):
        // a question-mark-in-circle at the tab-row level, right-aligned.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let help = ui.add(
                egui::Button::new(egui::RichText::new("?").strong())
                    .min_size(egui::vec2(22.0, 22.0))
                    .corner_radius(11.0),
            );
            let help = help.on_hover_text(t("Keyboard Shortcuts (F1)"));
            if help.clicked() {
                if app.shortcuts_help_open() {
                    app.close_shortcuts_help();
                } else {
                    app.open_shortcuts_help();
                }
            }
        });
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
            // Content-sized container (step 44.1, H52): the first
            // `ui.vertical` in a horizontal row absorbs the whole
            // remaining width in egui 0.37 and pushed every later group
            // off-window while its caption centred across the full row.
            // Measure the group with the same width formula the buttons
            // allocate with and reserve exactly that.
            let width = group_width(ui, group).max(120.0);
            let group_ui = ui.allocate_ui_with_layout(
                egui::vec2(width, 64.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.set_min_width(width);
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        for action in &group.actions {
                            let tip = command_tips.get(tip_index).map(String::as_str);
                            tip_index += 1;
                            let id = format!("ribbon.{}.{}.{}", tab.id, group.id, action.id);
                            if let Some(command) = ui
                                .push_id(id, |ui| {
                                    actions::action_button(app, ui, tab.id, group.id, action, tip)
                                })
                                .inner
                            {
                                intent = Some(command);
                            }
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
                },
            );
            layout_probe::record(
                &format!("ribbon.{}.{}", tab.id, group.id),
                group_ui.response.rect,
            );
            ui.separator();
        }
    });
    if let Some(command) = intent {
        crate::busy::dispatch_maybe_blocking(app, command);
    }
}

/// The painted width of one action's button — the same formula
/// [`ribbon_button_response`] allocates with — so a group container can
/// size itself to its content instead of the row's remaining width.
fn action_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::new(fonts::CONTROL.size, egui::FontFamily::Proportional);
    let measure = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::TRANSPARENT);
    (measure.size().x + 18.0).max(52.0)
}

/// Content width of a group: its buttons plus the row's item spacing
/// and a small padding margin.
fn group_width(ui: &egui::Ui, group: &RibbonGroup) -> f32 {
    let spacing = ui.spacing().item_spacing.x;
    let buttons: f32 = group
        .actions
        .iter()
        .map(|action| action_width(ui, t(action.label)))
        .sum();
    let gaps = group.actions.len().saturating_sub(1) as f32 * spacing;
    buttons + gaps + 8.0
}

/// An Access-sized command button (icon above the label). `hover` is the
/// already-localized tooltip (registry text, plus the disabled reason when
/// applicable — step 68.2, M74); `active` marks a window toggle that is
/// open (step 69.2, M79).
fn ribbon_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    tip: Option<&str>,
    hover: &str,
    active: bool,
) -> bool {
    ribbon_button_response(ui, label, icon, true, tip, hover, active).clicked()
}

/// The disabled twin: painted muted, hovers with the phase hint.
fn ribbon_button_disabled(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    tip: Option<&str>,
    hover: &str,
    active: bool,
) -> egui::Response {
    ribbon_button_response(ui, label, icon, false, tip, hover, active)
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
    hover: &str,
    active: bool,
) -> egui::Response {
    let tokens = tokens(ui.visuals());
    let width = action_width(ui, label);
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
    // An open window toggle reads as pressed: accent fill and border
    // (step 69.2, item M79).
    let (fill, stroke) = if active && enabled {
        (tokens.accent_fill, egui::Stroke::new(1.0, tokens.accent))
    } else {
        (visuals.weak_bg_fill, visuals.bg_stroke)
    };
    ui.painter().rect(
        rect,
        GEOMETRY.widget_corner_radius,
        fill,
        stroke,
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

    let font = egui::FontId::new(fonts::CONTROL.size, egui::FontFamily::Proportional);
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
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, active, label));
    response.on_hover_text(hover)
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
        "import_file" => Some(Command::ToggleImportWindow),
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
pub(crate) fn next_form_name(app: &App) -> String {
    next_name(
        app.forms().iter().map(|f| f.name.to_lowercase()).collect(),
        "Form",
    )
}

/// First free "ReportN" name (case-insensitive against the snapshot).
pub(crate) fn next_report_name(app: &App) -> String {
    next_name(
        app.reports()
            .iter()
            .map(|r| r.name.to_lowercase())
            .collect(),
        "Report",
    )
}

/// First free "QueryN" name (case-insensitive against the snapshot).
pub(crate) fn next_query_name(app: &App) -> String {
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
    use datacraft_core::ribbon::{ActionState, ribbon_spec};

    /// Seam contract (LESSONS #2): every enabled spec action except the
    /// input-bound `open_database`, the state-bound `view_mode` and the
    /// two-command table flows (`table`, `table_design`) maps to a
    /// command; nothing else does.
    #[test]
    fn every_enabled_action_maps_to_a_command() {
        let app = App::new(Box::new(MockPort::new()));
        for tab in rendered_tabs(Some("table")) {
            for group in &tab.groups {
                for action in &group.actions {
                    let mapped = command_for(action.id, &app);
                    match &action.state {
                        ActionState::Enabled
                            if !matches!(
                                action.id,
                                "open_database" | "view_mode" | "table" | "table_design"
                            ) =>
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
                ActionState::DisabledIn { .. } | ActionState::Planned => {
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
