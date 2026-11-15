//! One cell's zoom interactions (step 86.4, item M102) — split from
//! `grid.rs` for the file-size gate: Alt+click and a double click on a
//! read-only cell open the value viewer; the context menu offers "Zoom
//! Value". Returns whether a zoom fired (such a click never arms the
//! in-place editor).

use eframe::egui;

use crate::i18n::t;

pub(super) fn cell_zoom_interactions(
    ui: &egui::Ui,
    response: &egui::Response,
    row: usize,
    col: usize,
    edit_restricted: bool,
    on_zoom: &mut Option<&mut dyn FnMut(usize, usize)>,
) -> bool {
    let alt_click = response.clicked() && ui.input(|i| i.modifiers.alt);
    let dbl_readonly = response.double_clicked() && edit_restricted;
    let mut zoomed = false;
    if (alt_click || dbl_readonly)
        && let Some(callback) = on_zoom.as_mut()
    {
        callback(row, col);
        zoomed = true;
    }
    let mut zoom_menu: Option<(usize, usize)> = None;
    response.context_menu(|ui| {
        if ui.selectable_label(false, t("Zoom Value")).clicked() {
            zoom_menu = Some((row, col));
            ui.close();
        }
    });
    if let Some(target) = zoom_menu
        && let Some(callback) = on_zoom.as_mut()
    {
        callback(target.0, target.1);
        zoomed = true;
    }
    zoomed
}

/// The Shift+F2 zoom target: the selection's focus cell, clamped to the
/// loaded bounds (step 86.4, item M102).
pub(super) fn zoom_focus_target(
    selection: &datacraft_core::selection::Selection,
    body_len: usize,
    col_len: usize,
) -> (usize, usize) {
    let focus = selection.focus_cell();
    (
        focus.row.min(body_len.saturating_sub(1)),
        focus.col.min(col_len.saturating_sub(1)),
    )
}
