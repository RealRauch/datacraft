//! Tool-window placement helpers (step 69.7, item M85).
//!
//! egui opens windows top-left by default; the M42 review asked for a
//! centered initial position. Windows stay movable — the default position
//! applies on first open.

use eframe::egui;

/// A centered default position for a window of `size` on the current
/// screen content rect.
pub fn centered_pos(ctx: &egui::Context, size: egui::Vec2) -> egui::Pos2 {
    let screen = ctx.content_rect();
    egui::pos2(
        screen.center().x - size.x / 2.0,
        screen.center().y - size.y / 2.0,
    )
}
