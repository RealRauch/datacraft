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

/// One of the eight resize handles around a control rect (phase 102,
/// step 102.2; item M133).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    /// Top-left corner.
    NorthWest,
    /// Top edge.
    North,
    /// Top-right corner.
    NorthEast,
    /// Left edge.
    West,
    /// Right edge.
    East,
    /// Bottom-left corner.
    SouthWest,
    /// Bottom edge.
    South,
    /// Bottom-right corner.
    SouthEast,
}

impl ResizeHandle {
    /// Whether the handle moves the left edge.
    pub fn is_west(self) -> bool {
        matches!(self, Self::NorthWest | Self::West | Self::SouthWest)
    }

    /// Whether the handle moves the right edge.
    pub fn is_east(self) -> bool {
        matches!(self, Self::NorthEast | Self::East | Self::SouthEast)
    }

    /// Whether the handle moves the top edge.
    pub fn is_north(self) -> bool {
        matches!(self, Self::NorthWest | Self::North | Self::NorthEast)
    }

    /// Whether the handle moves the bottom edge.
    pub fn is_south(self) -> bool {
        matches!(self, Self::SouthWest | Self::South | Self::SouthEast)
    }

    /// Stable id for the UI's drag keys and paint ids.
    pub fn id(self) -> &'static str {
        match self {
            Self::NorthWest => "nw",
            Self::North => "n",
            Self::NorthEast => "ne",
            Self::West => "w",
            Self::East => "e",
            Self::SouthWest => "sw",
            Self::South => "s",
            Self::SouthEast => "se",
        }
    }
}

/// Side length of the painted handle square, in canvas points.
pub const HANDLE_SIZE: f32 = 8.0;

/// The eight handle centers in canvas points (corners first, then edges;
/// the designers paint a square of [`HANDLE_SIZE`] around each).
pub fn handle_centers(rect: Rect) -> [(ResizeHandle, (f32, f32)); 8] {
    let mid_x = rect.x + rect.width / 2.0;
    let mid_y = rect.y + rect.height / 2.0;
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    [
        (ResizeHandle::NorthWest, (rect.x, rect.y)),
        (ResizeHandle::North, (mid_x, rect.y)),
        (ResizeHandle::NorthEast, (right, rect.y)),
        (ResizeHandle::West, (rect.x, mid_y)),
        (ResizeHandle::East, (right, mid_y)),
        (ResizeHandle::SouthWest, (rect.x, bottom)),
        (ResizeHandle::South, (mid_x, bottom)),
        (ResizeHandle::SouthEast, (right, bottom)),
    ]
}

/// The handle under `(x, y)` — a square of [`HANDLE_SIZE`] around a
/// center. Corners win over edges when a control is tiny and they
/// overlap (the first match in [`handle_centers`] order).
pub fn handle_at(rect: Rect, x: f32, y: f32) -> Option<ResizeHandle> {
    let reach = HANDLE_SIZE / 2.0;
    handle_centers(rect)
        .into_iter()
        .find_map(|(handle, (cx, cy))| {
            ((x - cx).abs() <= reach && (y - cy).abs() <= reach).then_some(handle)
        })
}

/// Applies one resize gesture: the grabbed edges move by `(dx, dy)`, the
/// opposite edges stay, every edge snaps to the grid, a control never
/// shrinks below one grid step and the result never leaves `bounds`
/// (`(width, height)` of the canvas or page).
pub fn resize_rect(
    rect: Rect,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    grid: f32,
    bounds: (f32, f32),
) -> Rect {
    let grid = if grid.is_finite() && grid > 0.0 {
        grid
    } else {
        1.0
    };
    // The far edges clamp to the last grid line inside the bounds — a page
    // whose size is not a multiple of the grid (A4 = 595 pt) still yields
    // a valid, grid-aligned rect.
    let max_x = floor_grid(bounds.0, grid);
    let max_y = floor_grid(bounds.1, grid);
    let mut left = rect.x + if handle.is_west() { dx } else { 0.0 };
    let mut right = rect.x + rect.width + if handle.is_east() { dx } else { 0.0 };
    let mut top = rect.y + if handle.is_north() { dy } else { 0.0 };
    let mut bottom = rect.y + rect.height + if handle.is_south() { dy } else { 0.0 };
    left = snap(left, grid).min(max_x);
    right = snap(right, grid).min(max_x);
    top = snap(top, grid).min(max_y);
    bottom = snap(bottom, grid).min(max_y);
    // The grabbed edge yields at the minimum size; the anchored edge stays.
    if right - left < grid {
        if handle.is_west() {
            left = right - grid;
        } else {
            right = left + grid;
        }
    }
    if bottom - top < grid {
        if handle.is_north() {
            top = bottom - grid;
        } else {
            bottom = top + grid;
        }
    }
    // A minimum-size correction at the far edge must not leave the bounds.
    if right > max_x {
        right = max_x;
        left = (right - grid).max(0.0);
    }
    if bottom > max_y {
        bottom = max_y;
        top = (bottom - grid).max(0.0);
    }
    Rect {
        x: left.max(0.0),
        y: top.max(0.0),
        width: (right - left).max(grid),
        height: (bottom - top).max(grid),
    }
}

/// Largest grid multiple at or below `value` (clamped non-negative).
fn floor_grid(value: f32, grid: f32) -> f32 {
    (value / grid).floor().max(0.0) * grid
}
