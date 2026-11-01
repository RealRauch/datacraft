//! WYSIWYG canvas geometry (step 8.1, item H28).
//!
//! Pure, window-free math: snap-to-grid, rect geometry and hit-testing.
//! The form designer (UI) only consumes results — no layout logic in
//! paint code (ARCHITECTURE §8).

use serde::{Deserialize, Serialize};

/// A rectangle in canvas points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width (positive).
    pub width: f32,
    /// Height (positive).
    pub height: f32,
}

/// Snaps a coordinate to the grid (round-half-up, clamped non-negative).
pub fn snap(value: f32, grid: f32) -> f32 {
    ((value / grid).round() * grid).max(0.0)
}

/// Snaps a rect's origin and size to the grid (sizes stay positive).
pub fn snap_rect(rect: Rect, grid: f32) -> Rect {
    Rect {
        x: snap(rect.x, grid),
        y: snap(rect.y, grid),
        width: snap(rect.width, grid).max(grid),
        height: snap(rect.height, grid).max(grid),
    }
}

/// Index of the topmost rect containing the point, `None` outside all.
/// Later rects stack above earlier ones.
pub fn hit_test(rects: &[Rect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .rposition(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
}
