//! Contract suite for the shared grid core (step 6.1, item H22).
//!
//! ARCHITECTURE §8: layout math is pure and lives in core — the datasheet
//! (6.2) and the design grid (6.3) share it so virtual scrolling behaves
//! identically in both views. No window needed to test any of this.

use datacraft_core::grid::{
    DEFAULT_COLUMN_WIDTH, GridMetrics, MIN_COLUMN_WIDTH, clamp_scroll, column_at_x, column_span_x,
    effective_widths, ensure_row_visible, page_rows, resize_column, resize_handle_at,
    visible_row_range,
};
use proptest::prelude::*;

#[test]
fn scroll_follow_keeps_a_visible_row_in_place() {
    let metrics = GridMetrics::default();
    // Row 5 spans 110..132 px; a window starting at 88 px sees it whole.
    assert_eq!(ensure_row_visible(88.0, 400.0, 5, 1_000, &metrics), 88.0);
}

#[test]
fn scroll_follow_moves_up_for_rows_above_the_window() {
    let metrics = GridMetrics::default();
    // Row 2 starts at 44 px, above a window at 220 px: the window jumps up.
    assert_eq!(ensure_row_visible(220.0, 400.0, 2, 1_000, &metrics), 44.0);
}

#[test]
fn scroll_follow_moves_down_for_rows_below_the_window() {
    let metrics = GridMetrics::default();
    // Row 60 ends at 1_364 px, below a 400 px window at 0 px: minimal
    // scroll brings its bottom edge to the viewport bottom.
    assert_eq!(ensure_row_visible(0.0, 400.0, 60, 1_000, &metrics), 942.0);
}

#[test]
fn scroll_follow_clamps_at_both_scroll_ends() {
    let metrics = GridMetrics::default();
    // End key on the last row pins the window to the content end.
    assert_eq!(ensure_row_visible(0.0, 400.0, 999, 1_000, &metrics), 21_600.0);
    // Home key on the first row can never go negative.
    assert_eq!(ensure_row_visible(500.0, 400.0, 0, 1_000, &metrics), 0.0);
    // Clamped again on a second pass: follow is a fixpoint.
    let once = ensure_row_visible(0.0, 400.0, 999, 1_000, &metrics);
    assert_eq!(ensure_row_visible(once, 400.0, 999, 1_000, &metrics), once);
}

#[test]
fn page_rows_spans_the_viewport_and_never_collapses() {
    let metrics = GridMetrics::default();
    // 400 px viewport, 22 px rows -> 18 whole rows per page jump.
    assert_eq!(page_rows(400.0, &metrics), 18);
    // A tiny viewport still moves at least one row.
    assert_eq!(page_rows(10.0, &metrics), 1);
    assert_eq!(page_rows(0.0, &metrics), 1);
}

#[test]
fn empty_grids_show_nothing() {
    let metrics = GridMetrics::default();
    assert_eq!(visible_row_range(0.0, 400.0, 0, &metrics), 0..0);
    assert_eq!(clamp_scroll(100.0, 400.0, 0, &metrics), 0.0);
}

#[test]
fn visible_range_covers_the_viewport_plus_one_partial_row() {
    let metrics = GridMetrics::default();
    // 400 px viewport, 22 px rows -> 19 fully visible, +1 partial.
    assert_eq!(visible_row_range(0.0, 400.0, 1_000, &metrics), 0..19);
    // Scrolling one full row down shifts the window by one.
    assert_eq!(visible_row_range(22.0, 400.0, 1_000, &metrics), 1..20);
}

#[test]
fn visible_range_clamps_to_the_row_count() {
    let metrics = GridMetrics::default();
    assert_eq!(visible_row_range(0.0, 400.0, 5, &metrics), 0..5);
    assert_eq!(visible_row_range(10_000.0, 400.0, 5, &metrics), 5..5);
}

#[test]
fn scroll_clamps_to_the_content_height() {
    let metrics = GridMetrics::default();
    // 1000 rows * 22 px = 22000 px content; 400 px viewport.
    assert_eq!(clamp_scroll(-50.0, 400.0, 1_000, &metrics), 0.0);
    assert_eq!(clamp_scroll(30_000.0, 400.0, 1_000, &metrics), 21_600.0);
    // Content smaller than the viewport never scrolls.
    assert_eq!(clamp_scroll(50.0, 400.0, 10, &metrics), 0.0);
}

#[test]
fn clamped_scroll_is_a_fixpoint() {
    let metrics = GridMetrics::default();
    let once = clamp_scroll(30_000.0, 400.0, 1_000, &metrics);
    assert_eq!(clamp_scroll(once, 400.0, 1_000, &metrics), once);
}

#[test]
fn column_hit_testing_matches_span_geometry() {
    let widths = [80.0, 120.0, 40.0];
    assert_eq!(column_at_x(0.0, &widths), Some(0));
    assert_eq!(column_at_x(79.9, &widths), Some(0));
    assert_eq!(column_at_x(80.0, &widths), Some(1));
    assert_eq!(column_at_x(199.9, &widths), Some(1));
    assert_eq!(column_at_x(200.0, &widths), Some(2));
    assert_eq!(column_at_x(240.0, &widths), None, "past the last column");
    assert_eq!(column_at_x(0.0, &[]), None, "no columns");
    // Span geometry is the exact inverse: every hit column contains its x.
    for probe in [0.0, 80.0, 199.9, 239.9] {
        if let Some(col) = column_at_x(probe, &widths) {
            let (x, w) = column_span_x(col, &widths);
            assert!(probe >= x && probe < x + w, "{probe} not in column {col}");
        }
    }
    assert_eq!(column_span_x(0, &widths), (0.0, 80.0));
    assert_eq!(column_span_x(2, &widths), (200.0, 40.0));
}

#[test]
fn resize_handles_sit_on_column_right_edges() {
    let widths = [80.0, 120.0, 40.0];
    assert_eq!(resize_handle_at(80.0, &widths, 3.0), Some(0));
    assert_eq!(resize_handle_at(82.9, &widths, 3.0), Some(0));
    assert_eq!(resize_handle_at(77.2, &widths, 3.0), Some(0));
    assert_eq!(resize_handle_at(199.0, &widths, 3.0), Some(1));
    assert_eq!(
        resize_handle_at(240.0, &widths, 3.0),
        Some(2),
        "the last right edge resizes the last column"
    );
    assert_eq!(resize_handle_at(84.0, &widths, 3.0), None);
    assert_eq!(resize_handle_at(0.0, &widths, 3.0), None);
    assert_eq!(resize_handle_at(80.0, &[], 3.0), None);
}

#[test]
fn resized_columns_stop_at_the_minimum() {
    assert_eq!(resize_column(140.0, -30.0), 110.0);
    assert_eq!(resize_column(30.0, -100.0), MIN_COLUMN_WIDTH);
    assert_eq!(resize_column(MIN_COLUMN_WIDTH, -1.0), MIN_COLUMN_WIDTH);
}

#[test]
fn effective_widths_pad_new_columns_and_drop_stale_ones() {
    assert_eq!(
        effective_widths(&[], 2, DEFAULT_COLUMN_WIDTH),
        [DEFAULT_COLUMN_WIDTH; 2]
    );
    assert_eq!(
        effective_widths(&[90.0], 3, DEFAULT_COLUMN_WIDTH),
        [90.0, DEFAULT_COLUMN_WIDTH, DEFAULT_COLUMN_WIDTH]
    );
    assert_eq!(
        effective_widths(&[90.0, 100.0], 1, DEFAULT_COLUMN_WIDTH),
        [90.0],
        "widths of removed columns do not linger"
    );
}

proptest! {
    /// The visible window always lies inside 0..row_count and is ordered.
    #[test]
    fn visible_range_stays_inside_the_row_set(
        scroll_y in 0.0f32..1_000_000.0,
        viewport in 1.0f32..4_000.0,
        rows in 0usize..10_000,
    ) {
        let metrics = GridMetrics::default();
        let range = visible_row_range(scroll_y, viewport, rows, &metrics);
        prop_assert!(range.start <= range.end);
        prop_assert!(range.end <= rows);
        prop_assert!(range.start <= rows);
    }

    /// Hit-testing an x inside the columns' total width always succeeds and
    /// the span of that column contains the probe.
    #[test]
    fn hit_test_and_span_are_inverses(
        widths in proptest::collection::vec(1.0f32..500.0, 1..24),
        ratio in 0.0f32..1.0,
    ) {
        let total: f32 = widths.iter().sum();
        let x = total * ratio;
        if let Some(col) = column_at_x(x, &widths) {
            let (cx, cw) = column_span_x(col, &widths);
            prop_assert!(x >= cx && x < cx + cw);
        } else {
            prop_assert!(x >= total);
        }
    }

    /// Scroll clamping never returns a negative or over-content offset.
    #[test]
    fn clamped_scroll_is_never_negative(
        scroll_y in -1_000.0f32..1_000_000.0,
        viewport in 1.0f32..4_000.0,
        rows in 0usize..10_000,
    ) {
        let metrics = GridMetrics::default();
        let clamped = clamp_scroll(scroll_y, viewport, rows, &metrics);
        prop_assert!(clamped >= 0.0);
        let content = rows as f32 * metrics.row_height;
        prop_assert!(clamped <= (content - viewport).max(0.0));
    }

    /// Scroll follow always leaves the target row inside the visible
    /// window (when the row fits the viewport) and yields a clamped,
    /// idempotent offset — step 40.1, item M38.
    #[test]
    fn scroll_follow_places_the_row_inside_the_window(
        scroll_y in -1_000.0f32..1_000_000.0,
        viewport in 44.0f32..4_000.0,
        rows in 1usize..10_000,
        row in 0usize..10_000,
    ) {
        prop_assume!(row < rows, "the UI never navigates past the last row");
        let metrics = GridMetrics::default();
        let followed = ensure_row_visible(scroll_y, viewport, row, rows, &metrics);
        prop_assert_eq!(clamp_scroll(followed, viewport, rows, &metrics), followed);
        let top = row as f32 * metrics.row_height;
        let bottom = top + metrics.row_height;
        prop_assert!(followed <= top, "window starts past the row top");
        prop_assert!(
            bottom <= followed + viewport + 0.01,
            "row bottom {bottom} falls below the window end {}",
            followed + viewport
        );
    }
}
