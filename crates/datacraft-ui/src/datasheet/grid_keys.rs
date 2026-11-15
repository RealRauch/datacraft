//! The shared grid's per-frame key reading (split from `grid.rs` for the
//! file-size gate, step 87.1 — the L30 band rule): the navigation gate,
//! the focus lock and every consumed key intent in one pass.
//! Paint-free: `grid_ui` applies the struct's intents after the body
//! pass.

use eframe::egui;

use datacraft_core::selection::Selection;

use super::nav::{
    NavIntent, read_copy_key, read_delete_key, read_fill_key, read_fk_follow_key, read_nav_intent,
    read_paste_key, read_select_all, read_zoom_key,
};

/// Every key intent one grid pass consumes: `nav` carries the record
/// navigation (the second field reports a Shift-modified press, the
/// selection-extension variant, step 86.1/M101); the clipboard, fill and
/// clear intents arrive pre-gated on the caller's edit restrictions;
/// `follow_key` is the F4 affordance (step 87.1, M103 — the caller's
/// per-column labels gate it); a whole-column or select-all selection
/// always extends (`extended`), a collapsed cell keeps the record
/// deletion (step 40.3, M44).
pub(crate) struct GridKeys {
    pub nav: Option<(NavIntent, bool)>,
    pub select_all: bool,
    pub copy: bool,
    pub paste: bool,
    pub fill: bool,
    pub zoom_key: bool,
    pub follow_key: bool,
    pub clear_intent: bool,
    pub delete_intent: bool,
}

/// Reads the navigation gate and every key of this frame, consumed so no
/// other handler reacts to them twice. `delete_allowed` reflects the
/// source's record-deletion capability — a restricted or locked grid
/// never consumes Del as a deletion intent.
pub(crate) fn grid_keys(
    ui: &egui::Ui,
    anchor_id: egui::Id,
    body_len: usize,
    selection: &Selection,
    edit_restricted: bool,
    locked: bool,
    delete_allowed: bool,
) -> GridKeys {
    // Navigation gate: the grid reacts when its focus anchor holds focus
    // (a row-selector click arms it) or when the pointer hovers it —
    // unless another widget, e.g. a text field, holds focus. Focus is
    // read live; hover uses the previous frame's anchor rect.
    let focused_elsewhere = ui
        .ctx()
        .memory(|memory| memory.focused().is_some_and(|id| id != anchor_id));
    let hover_armed = ui
        .ctx()
        .read_response(anchor_id)
        .is_some_and(|response| response.hovered());
    let wants_nav = ui.ctx().memory(|memory| memory.has_focus(anchor_id))
        || (hover_armed && !focused_elsewhere);
    // egui would otherwise move focus to the next focusable widget on the
    // first arrow press; the lock keeps the anchor focused so the whole
    // walk stays on the grid. Tab still leaves it (accessible), Esc still
    // reaches the global cancel-edit path.
    if ui.ctx().memory(|memory| memory.has_focus(anchor_id)) {
        ui.ctx().memory_mut(|memory| {
            memory.set_focus_lock_filter(
                anchor_id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..egui::EventFilter::default()
                },
            );
        });
    }
    let nav = if wants_nav { read_nav_intent(ui) } else { None };
    // Ctrl+A selects everything loaded (step 86.1, item M101); the
    // wants_nav gate keeps it away from text fields holding focus.
    let select_all = wants_nav && read_select_all(ui);
    // Clipboard intents (step 86.2, item M101): Ctrl+C copies (read-only),
    // Ctrl+V pastes only on writable, unlocked grids.
    let copy = wants_nav && body_len > 0 && read_copy_key(ui);
    let paste = wants_nav && !edit_restricted && !locked && read_paste_key(ui);
    let fill = wants_nav && !edit_restricted && !locked && read_fill_key(ui);
    // Shift+F2 zooms the selection's focus cell (step 86.4, item M102);
    // readable even while the in-place editor holds focus.
    let zoom_key = body_len > 0 && read_zoom_key(ui);
    // F4 follows the focus cell's first target (step 87.1, item M103);
    // the caller's per-column labels gate the intent.
    let follow_key = wants_nav && body_len > 0 && read_fk_follow_key(ui);
    // A whole-column or select-all selection always extends; a collapsed
    // cell keeps the step 40.3 record deletion.
    let extended = !matches!(selection, Selection::Cell { .. });
    // Del on the selected record is a deletion intent, not a cursor move
    // (step 40.3, item M44); a submission in flight refuses it (85.2/H61).
    let del = wants_nav && delete_allowed && !locked && read_delete_key(ui);
    let clear_intent = del && extended && body_len > 0;
    let delete_intent = del && !extended && body_len > 0;
    GridKeys {
        nav,
        select_all,
        copy,
        paste,
        fill,
        zoom_key,
        follow_key,
        clear_intent,
        delete_intent,
    }
}
