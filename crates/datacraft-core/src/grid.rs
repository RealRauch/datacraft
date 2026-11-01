//! Shared grid core (step 6.1, item H22; ARCHITECTURE §8).
//!
//! Pure, window-free layout math for virtualized grids: the datasheet
//! (step 6.2) and the design grid (step 6.3) share these functions, so
//! virtual scrolling behaves identically in both views and is testable
//! without a window. Paint code only consumes the results.

/// Row metrics of a grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridMetrics {
    /// Height of one data row in points.
    pub row_height: f32,
    /// Height of the header row in points.
    pub header_height: f32,
}

impl Default for GridMetrics {
    fn default() -> Self {
        Self {
            row_height: 22.0,
            header_height: 26.0,
        }
    }
}

/// Visible row window for a viewport at the given scroll offset: the first
/// visible row and the exclusive end, covering fully visible rows plus one
/// partial row at the bottom edge. Empty grids and out-of-range scrolls
/// yield an empty window.
pub fn visible_row_range(
    scroll_y: f32,
    viewport_height: f32,
    row_count: usize,
    metrics: &GridMetrics,
) -> std::ops::Range<usize> {
    if row_count == 0 {
        return 0..0;
    }
    let first = ((scroll_y / metrics.row_height).floor().max(0.0) as usize).min(row_count);
    // Ceiling covers fully visible rows plus the partial one at the edge.
    let visible = (viewport_height / metrics.row_height).ceil() as usize;
    let end = (first + visible).min(row_count);
    first..end
}

/// Clamps a scroll offset to the content height: never negative, never past
/// the last row; content smaller than the viewport does not scroll.
pub fn clamp_scroll(
    scroll_y: f32,
    viewport_height: f32,
    row_count: usize,
    metrics: &GridMetrics,
) -> f32 {
    let content = row_count as f32 * metrics.row_height;
    let max = (content - viewport_height).max(0.0);
    scroll_y.clamp(0.0, max)
}

/// Column hit test: the column whose span contains `x`, or `None` past the
/// last column. Zero-width leading columns are skipped naturally.
pub fn column_at_x(x: f32, widths: &[f32]) -> Option<usize> {
    let mut acc = 0.0;
    for (i, w) in widths.iter().enumerate() {
        acc += w;
        if x < acc {
            return Some(i);
        }
    }
    None
}

/// X offset and width of column `col` — the exact inverse of
/// [`column_at_x`] for valid columns; out-of-range columns span nothing
/// (`(total, 0.0)`).
pub fn column_span_x(col: usize, widths: &[f32]) -> (f32, f32) {
    let mut acc = 0.0;
    for (i, w) in widths.iter().enumerate() {
        if i == col {
            return (acc, *w);
        }
        acc += w;
    }
    (acc, 0.0)
}
