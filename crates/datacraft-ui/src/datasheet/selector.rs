//! The grid's row-selector column painting (step 6.2, H23; record menu
//! step 40.3, item M44).
//!
//! Split from `grid.rs` for the file-size gate (step 86.2). The selector
//! is an interact+paint pair, not a laid-out Button: `ui.put` would clamp
//! the rect against the scrolled content ui's max_rect and break the
//! pinned placement. AccessKit still reports it as a Button with the row
//! number.

use eframe::egui;

use datacraft_core::availability::DisabledReason;

use super::grid::SELECTOR_WIDTH;
use crate::i18n::t;
use crate::theme::Tokens;

/// Paints one row's record selector: the pinned number cell, the cursor
/// highlight, the click that selects the record (reported through
/// `clicked_row`) and the record menu whose Delete entry reports through
/// `delete_row`; for restricted sources it renders disabled with the
/// reason, and a submission in flight refuses with its own note (step
/// 85.2, item H61).
#[allow(clippy::too_many_arguments)]
pub(super) fn record_selector(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    row: usize,
    is_cursor: bool,
    delete_reason: Option<&DisabledReason>,
    locked: bool,
    tokens: &Tokens,
    font: &egui::FontId,
    clicked_row: &mut Option<usize>,
    delete_row: &mut Option<usize>,
) {
    let selector_label = format!("{:>6}", row + 1);
    let selector_id = ui.scope_id().with(("selector", row));
    let selector = ui.interact(rect, selector_id, egui::Sense::click());
    selector.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, true, selector_label.clone())
    });
    if is_cursor {
        ui.painter().rect_filled(rect, 2.0, tokens.accent_faint);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        selector_label,
        font.clone(),
        if is_cursor {
            tokens.accent_text
        } else {
            tokens.text
        },
    );
    if selector.clicked() {
        *clicked_row = Some(row);
    }
    selector.context_menu(|ui| match (delete_reason, locked) {
        (Some(reason), _) => {
            ui.add_enabled(false, egui::Button::new(t("Delete Record")))
                .on_hover_text(crate::availability::reason_text(reason));
        }
        (None, true) => {
            ui.add_enabled(false, egui::Button::new(t("Delete Record")))
                .on_hover_text(t("A submission is in flight — apply or discard it first."));
        }
        (None, false) => {
            if ui.button(t("Delete Record")).clicked() {
                *delete_row = Some(row);
                ui.close();
            }
        }
    });
}

/// The selector's rect for display row `row` (pinned at the viewport's
/// left edge by the scroll compensation).
pub(super) fn selector_rect(origin: egui::Pos2, scroll_x: f32, row_height: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        origin + egui::vec2(scroll_x, 0.0),
        egui::vec2(SELECTOR_WIDTH, row_height),
    )
}
