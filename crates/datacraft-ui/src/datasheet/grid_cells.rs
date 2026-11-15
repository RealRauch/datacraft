//! One cell's zoom and follow interactions (steps 86.4/87.1, items
//! M102/M103) — split from `grid.rs` for the file-size gate: Alt+click
//! and a double click on a read-only cell open the value viewer; the
//! context menu offers "Zoom Value" plus the column's follow targets.
//! Returns whether an interaction fired (such a click never arms the
//! in-place editor).

use eframe::egui;

use crate::i18n::t;

#[allow(clippy::too_many_arguments)]
pub(super) fn cell_zoom_interactions(
    ui: &egui::Ui,
    response: &egui::Response,
    row: usize,
    col: usize,
    edit_restricted: bool,
    on_zoom: &mut Option<&mut dyn FnMut(usize, usize)>,
    follow_labels: &[String],
    on_follow: &mut Option<&mut dyn FnMut(usize, usize, usize)>,
) -> bool {
    let alt_click = response.clicked() && ui.input(|i| i.modifiers.alt);
    let dbl_readonly = response.double_clicked() && edit_restricted;
    let mut fired = false;
    if (alt_click || dbl_readonly)
        && let Some(callback) = on_zoom.as_mut()
    {
        callback(row, col);
        fired = true;
    }
    let mut zoom_menu: Option<(usize, usize)> = None;
    let mut follow: Option<usize> = None;
    response.context_menu(|ui| {
        if ui.selectable_label(false, t("Zoom Value")).clicked() {
            zoom_menu = Some((row, col));
            ui.close();
            return;
        }
        if follow_labels.is_empty() {
            return;
        }
        ui.separator();
        for (index, label) in follow_labels.iter().enumerate() {
            if ui.selectable_label(false, label).clicked() {
                follow = Some(index);
                ui.close();
                return;
            }
        }
    });
    if let Some(target) = zoom_menu
        && let Some(callback) = on_zoom.as_mut()
    {
        callback(target.0, target.1);
        fired = true;
    }
    if let Some(index) = follow
        && let Some(callback) = on_follow.as_mut()
    {
        callback(row, col, index);
        fired = true;
    }
    fired
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

/// The F4 follow target (step 87.1, item M103): the selection's focus
/// cell, only when its column carries follow targets.
pub(super) fn follow_key_target(
    selection: &datacraft_core::selection::Selection,
    body_len: usize,
    follow_targets: &[Vec<String>],
) -> Option<(usize, usize)> {
    let focus = selection.focus_cell();
    let col = focus.col.min(follow_targets.len().saturating_sub(1));
    (focus.row < body_len
        && follow_targets
            .get(col)
            .is_some_and(|labels| !labels.is_empty()))
    .then_some((focus.row, col))
}
