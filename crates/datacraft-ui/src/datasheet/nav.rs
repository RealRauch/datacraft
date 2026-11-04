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
/// reacts to them twice.
pub(crate) fn read_nav_intent(ui: &egui::Ui) -> Option<NavIntent> {
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
            if input.consume_key(egui::Modifiers::NONE, key) {
                return Some(intent);
            }
        }
        None
    })
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
