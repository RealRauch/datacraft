//! The selection model's contract suite (step 86.1, item M101).
//!
//! Pure state checks: the model addresses displayed positions, normalizes
//! rectangles from either corner, and clamps per the documented survival
//! rule (scroll never changes a selection; a row/column count change
//! clamps both corners on the next read).

use datacraft_core::selection::{CellPos, Selection};

fn rect(anchor: (usize, usize), focus: (usize, usize)) -> Selection {
    Selection::Rect {
        anchor: CellPos {
            row: anchor.0,
            col: anchor.1,
        },
        focus: CellPos {
            row: focus.0,
            col: focus.1,
        },
    }
}

/// A collapsed selection is one cell; extending it forms a rectangle that
/// keeps the original cell as the anchor.
#[test]
fn extending_a_collapsed_selection_keeps_the_anchor() {
    let sel = Selection::collapse(2, 1);
    assert_eq!(sel, Selection::collapse(2, 1));

    let extended = sel.extend_to_cell(5, 3);
    assert_eq!(
        extended,
        rect((2, 1), (5, 3)),
        "the anchor cell survives the extension"
    );
    assert_eq!(extended.row_range(10), 2..6);
    assert_eq!(extended.col_range(4), 1..4);
}

/// Extending past the anchor (backwards) normalizes: the ranges cover the
/// rectangle regardless of which corner is the anchor.
#[test]
fn extending_backwards_normalizes_the_rectangle() {
    let sel = rect((5, 3), (5, 3)).extend_to_cell(0, 0);
    assert_eq!(sel.row_range(10), 0..6);
    assert_eq!(sel.col_range(4), 0..4);
}

/// Containment covers the closed rectangle, nothing outside it.
#[test]
fn containment_covers_the_rectangle_and_nothing_outside() {
    let sel = rect((2, 1), (5, 3));
    assert!(sel.contains(2, 1, 10, 4));
    assert!(sel.contains(5, 3, 10, 4));
    assert!(sel.contains(3, 2, 10, 4));
    assert!(!sel.contains(1, 1, 10, 4), "row above the rectangle");
    assert!(!sel.contains(6, 2, 10, 4), "row below the rectangle");
    assert!(!sel.contains(3, 0, 10, 4), "column left of the rectangle");
    assert!(!sel.contains(3, 4, 10, 4), "column right of the rectangle");
}

/// The survival rule for scroll: a selection is pure display state —
/// clamping against unchanged bounds returns it unchanged.
#[test]
fn clamping_to_unchanged_bounds_keeps_the_selection() {
    let sel = rect((2, 1), (5, 3));
    assert_eq!(sel.clamped(10, 4), sel);
}

/// The survival rule for sort/filter: when the row count shrinks, both
/// corners clamp into the new bounds; the rectangle shrinks with them.
#[test]
fn a_shrinking_row_count_clamps_both_corners() {
    let sel = rect((2, 1), (5, 3));
    let clamped = sel.clamped(4, 4);
    assert_eq!(clamped.row_range(4), 2..4, "the rectangle shrinks");
    assert_eq!(clamped.col_range(4), 1..4, "columns are unaffected");

    let tight = sel.clamped(2, 4);
    assert_eq!(tight.row_range(2), 1..2, "both corners clamp to the bound");
    assert_eq!(tight.col_range(4), 1..4);
}

/// A selection entirely outside the new bounds clamps to the nearest
/// in-bounds edge: a rectangle shrinks to the boundary intersection, a
/// cell moves onto the boundary — never to nothing.
#[test]
fn an_out_of_bounds_selection_clamps_to_the_nearest_edge() {
    let clamped = rect((8, 1), (9, 3)).clamped(4, 4);
    assert_eq!(clamped.row_range(4), 3..4, "the rectangle meets the boundary");
    assert_eq!(clamped.col_range(4), 1..4);
    assert_eq!(Selection::collapse(7, 1).clamped(4, 4), Selection::collapse(3, 1));
}

/// A column-count change (schema refresh) clamps columns the same way.
#[test]
fn a_shrinking_column_count_clamps_columns() {
    let sel = rect((2, 1), (5, 3));
    let clamped = sel.clamped(10, 3);
    assert_eq!(clamped.col_range(3), 1..3);
    assert_eq!(clamped.row_range(10), 2..6);
    assert_eq!(Selection::collapse(0, 2).clamped(10, 2), Selection::collapse(0, 1));
}

/// Column selection covers whole columns and extends in both directions
/// from its anchor.
#[test]
fn column_selection_extends_in_both_directions() {
    let sel = Selection::select_column(2);
    assert!(sel.contains(0, 2, 10, 4));
    assert!(sel.contains(9, 2, 10, 4));
    assert!(!sel.contains(0, 1, 10, 4));

    let left = sel.extend_columns(0);
    assert_eq!(left.col_range(4), 0..3);
    let right = sel.extend_columns(3);
    assert_eq!(right.col_range(4), 2..4);
}

/// Ctrl+A selects everything loaded; every cell and both full ranges
/// answer true.
#[test]
fn select_all_covers_every_loaded_cell() {
    let sel = Selection::select_all();
    assert!(sel.contains(0, 0, 3, 2));
    assert!(sel.contains(2, 1, 3, 2));
    assert_eq!(sel.row_range(3), 0..3);
    assert_eq!(sel.col_range(2), 0..2);
    // All is bounds-free: shrinking bounds keep it all.
    assert_eq!(sel.clamped(1, 1), Selection::select_all());
}

/// Shift+arrows extend from the current anchor; a column selection's
/// anchor cell is its anchor column's first row.
#[test]
fn the_anchor_cell_survives_every_selection_kind() {
    assert_eq!(Selection::collapse(2, 1).anchor_cell(), CellPos { row: 2, col: 1 });
    assert_eq!(rect((2, 1), (5, 3)).anchor_cell(), CellPos { row: 2, col: 1 });
    assert_eq!(
        Selection::select_column(2).anchor_cell(),
        CellPos { row: 0, col: 2 }
    );
    assert_eq!(Selection::select_all().anchor_cell(), CellPos { row: 0, col: 0 });
}
