//! Keyboard navigation of the shared grid (step 40.1, item M38).
//!
//! The record cursor is presentation state (`app.rs` — deliberately no
//! core command); these helpers map the navigation keys onto it and keep
//! the scroll-follow math pure in core `grid` (`ensure_row_visible`,
//! `page_rows`), so the datasheet, the query results and the SQL window
//! navigate and scroll identically. The grid reacts when its focus anchor
//! holds focus, or when the pointer hovers it and no other widget (e.g.
//! a text field) holds focus.

use eframe::egui;

use datacraft_core::grid::{GridMetrics, page_rows};

/// One navigation key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavIntent {
    First,
    Previous,
    Next,
    Last,
    PageUp,
    PageDown,
}

/// The navigation keys seen this frame, consumed so no other handler
/// reacts to them twice. The flag reports a Shift-modified press — the
/// selection-extension variant (step 86.1, item M101); a plain press
/// collapses the selection onto the new cursor row.
pub(crate) fn read_nav_intent(ui: &egui::Ui) -> Option<(NavIntent, bool)> {
    const KEYS: [(egui::Key, NavIntent); 6] = [
        (egui::Key::Home, NavIntent::First),
        (egui::Key::End, NavIntent::Last),
        (egui::Key::ArrowUp, NavIntent::Previous),
        (egui::Key::ArrowDown, NavIntent::Next),
        (egui::Key::PageUp, NavIntent::PageUp),
        (egui::Key::PageDown, NavIntent::PageDown),
    ];
    ui.ctx().input_mut(|input| {
        for (key, intent) in KEYS {
            // egui matches modifiers exactly (matches_logically), so the
            // shifted variant must be consumed before the plain one.
            if input.consume_key(egui::Modifiers::SHIFT, key) {
                return Some((intent, true));
            }
            if input.consume_key(egui::Modifiers::NONE, key) {
                return Some((intent, false));
            }
        }
        None
    })
}

/// Ctrl+A seen this frame (the select-all intent, step 86.1, item M101),
/// consumed so no other handler reacts to it twice.
pub(crate) fn read_select_all(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::A))
}

/// Ctrl+C seen this frame (the copy-selection intent, step 86.2, item
/// M101), consumed so no other handler reacts to it twice.
pub(crate) fn read_copy_key(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::C))
}

/// Ctrl+V seen this frame (the paste intent, step 86.2, item M101),
/// consumed so no other handler reacts to it twice.
pub(crate) fn read_paste_key(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::V))
}

/// Shift+F2 seen this frame (the value-viewer intent, step 86.4, item
/// M102), consumed so no other handler reacts to it twice.
pub(crate) fn read_zoom_key(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F2))
}

/// Ctrl+D seen this frame (the fill-down intent, step 86.3, item M101),
/// consumed so no other handler reacts to it twice.
pub(crate) fn read_fill_key(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::D))
}

/// The selection after a navigation key (step 86.1, item M101): a plain
/// press collapses onto the new cursor row, a Shift press extends from
/// the current anchor — `All` stays all. `focus_col` is the moving
/// corner's column ([`super`] uses `Selection::focus_cell`).
pub(crate) fn selection_after_nav(
    current: &datacraft_core::selection::Selection,
    target: usize,
    extend: bool,
    focus_col: usize,
) -> datacraft_core::selection::Selection {
    if extend {
        if matches!(current, datacraft_core::selection::Selection::All) {
            datacraft_core::selection::Selection::All
        } else {
            current.extend_to_cell(target, focus_col)
        }
    } else {
        datacraft_core::selection::Selection::collapse(target, focus_col)
    }
}

/// Next cursor row for an intent, clamped to the grid; empty grids stay
/// at row zero.
pub(crate) fn nav_target(
    intent: NavIntent,
    cursor: usize,
    row_count: usize,
    viewport_height: f32,
    metrics: &GridMetrics,
) -> usize {
    let last = row_count.saturating_sub(1);
    let page = page_rows(viewport_height, metrics);
    match intent {
        NavIntent::First => 0,
        NavIntent::Previous => cursor.saturating_sub(1),
        NavIntent::Next => (cursor + 1).min(last),
        NavIntent::Last => last,
        NavIntent::PageUp => cursor.saturating_sub(page).min(last),
        NavIntent::PageDown => (cursor + page).min(last),
    }
}

/// The Del key seen this frame (the selected record's deletion intent,
/// step 40.3, item M44), consumed so no other handler reacts to it twice.
pub(crate) fn read_delete_key(ui: &egui::Ui) -> bool {
    ui.ctx()
        .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Delete))
}
