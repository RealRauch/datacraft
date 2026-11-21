//! Nav-row painting (step 91.7, item L55; split step 92.4 for the size
//! gate, item L48's multi-select wash included).
//!
//! One interacted rect painted like the backstage nav rows: the
//! selection/active wash, the kind icon and the label. Pure paint — the
//! interaction intents stay with the caller (`nav_pane.rs`).

use eframe::egui;

use datacraft_core::ObjectKind;

use crate::theme::tokens;

/// Paints one nav row inside `rect`: the wash when the row is active or
/// part of the selection, the kind icon and the label.
pub(crate) fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    row_height: f32,
    name: &str,
    kind: ObjectKind,
    highlighted: bool,
) {
    let tokens = tokens(ui.visuals());
    if highlighted {
        ui.painter().rect_filled(rect, 0.0, tokens.accent_faint);
    }
    let icon_ink = if highlighted {
        tokens.accent_text
    } else {
        tokens.text_muted
    };
    let icon_rect = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(4.0, (row_height - crate::icons::ICON_SIZE) / 2.0),
        egui::vec2(crate::icons::ICON_SIZE, crate::icons::ICON_SIZE),
    );
    crate::icons::paint(
        ui.painter(),
        icon_rect,
        kind.slug(),
        icon_ink,
        tokens.accent,
    );
    let font = egui::TextStyle::Body.resolve(ui.style());
    let text_ink = if highlighted {
        tokens.accent_text
    } else {
        tokens.text
    };
    ui.painter().text(
        rect.left_top() + egui::vec2(crate::icons::ICON_SIZE + 10.0, row_height / 2.0),
        egui::Align2::LEFT_CENTER,
        name,
        font,
        text_ink,
    );
}
