//! Contract suite for the shared grid core (step 6.1, item H22).
//!
//! ARCHITECTURE §8: layout math is pure and lives in core — the datasheet
//! (6.2) and the design grid (6.3) share it so virtual scrolling behaves
//! identically in both views. No window needed to test any of this.

use datacraft_core::grid::{GridMetrics, clamp_scroll, column_at_x, column_span_x, visible_row_range};
use proptest::prelude::*;

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
}
