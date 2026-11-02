//! Quick Access Toolbar (step 21.2, item U6).
//!
//! A compact icon strip above the ribbon, spec-driven from core. Entries
//! reuse the ribbon action ids, so both surfaces dispatch through the same
//! `command_for` mapping; disabled entries name their follow-up item.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{ActionState, ribbon_spec};

use crate::app::App;
use crate::i18n::{t, tr};
use crate::icons;
use crate::theme::GEOMETRY;
use crate::theme::tokens;

/// Renders the QAT strip and returns the single intent it collected.
pub(super) fn quick_access_ui(app: &App, ui: &mut egui::Ui) -> Option<Command> {
    let mut intent = None;
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        for entry in &ribbon_spec().quick_access {
            ui.push_id(format!("qat.{}", entry.id), |ui| match &entry.state {
                ActionState::Enabled => {
                    // Undo/Redo enable from the snapshot's history depth
                    // (step 28.1, item U14); every other entry stays on.
                    let enabled = match entry.id {
                        "undo" => app.snapshot().undo_depth > 0,
                        "redo" => app.snapshot().redo_depth > 0,
                        _ => true,
                    };
                    let clicked = qat_button(ui, t(entry.label), entry.id, enabled).clicked();
                    if clicked
                        && enabled
                        && let Some(command) = super::command_for(entry.id, app)
                    {
                        intent = Some(command);
                    }
                }
                ActionState::DisabledIn { phase } => {
                    qat_button(ui, t(entry.label), entry.id, false)
                        .on_hover_text(tr("arrives in {phase}", &[("phase", phase)]));
                }
            });
        }
    });
    ui.add(egui::Separator::default().horizontal());
    intent
}

/// A compact QAT button (icon only); the AccessKit label keeps
/// label-based addressing intact, the tooltip carries the localized name
/// and the follow-up item for disabled entries.
fn qat_button(ui: &mut egui::Ui, label: &str, icon: &str, enabled: bool) -> egui::Response {
    let tokens = tokens(ui.visuals());
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(24.0, 22.0), sense);
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
        rect.center(),
        egui::vec2(icons::ICON_SIZE, icons::ICON_SIZE),
    );
    icons::paint(ui.painter(), icon_rect, icon, ink, accent);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    response.on_hover_text(label)
}
