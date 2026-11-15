//! The clipboard TSV vocabulary's contract suite (step 86.2, item M101).
//!
//! Copy builds a spreadsheet-compatible TSV over the selection; paste
//! parses a matrix with symmetric unquoting and validates size before
//! any target exists; `paste_targets` maps the matrix onto the selection
//! or answers a precise rejection — so the UI can reject a paste before
//! any cell changes.

use datacraft_core::clipboard::{self, MAX_TSV_CELLS};
use datacraft_core::selection::{CellPos, Selection};

fn body() -> Vec<Vec<String>> {
    vec![
        vec!["1".into(), "alpha".into()],
        vec!["2".into(), "be\ta".into()],
        vec!["3".into(), "line\nbreak".into()],
        vec!["4".into(), "quote\"d".into()],
    ]
}

/// A collapsed selection copies exactly one cell.
#[test]
fn a_collapsed_selection_copies_one_cell() {
    let tsv = clipboard::copy_tsv(&body(), &Selection::collapse(1, 0), 4, 2).unwrap();
    assert_eq!(tsv, "2");
}

/// A rectangle copies tab-separated cells and newline-separated rows; a
/// cell containing a tab is quoted so the text stays parseable.
#[test]
fn a_rectangle_copies_tab_separated_rows() {
    let sel = Selection::Rect {
        anchor: CellPos { row: 0, col: 0 },
        focus: CellPos { row: 1, col: 1 },
    };
    let tsv = clipboard::copy_tsv(&body(), &sel, 4, 2).unwrap();
    assert_eq!(tsv, "1\talpha\n2\t\"be\ta\"");
}

/// Values containing tabs, newlines or quotes are Excel-style quoted so
/// the text round-trips through a spreadsheet.
#[test]
fn special_values_are_quoted_and_round_trip() {
    let tsv = clipboard::copy_tsv(&body(), &Selection::select_all(), 4, 2).unwrap();
    let matrix = clipboard::parse_tsv(&tsv).unwrap();
    assert_eq!(matrix[1][1], "be\ta");
    assert_eq!(matrix[2][1], "line\nbreak");
    assert_eq!(matrix[3][1], "quote\"d");
}

/// A column selection copies every loaded row of the selected columns.
#[test]
fn a_column_selection_copies_whole_columns() {
    let sel = Selection::select_column(0);
    let tsv = clipboard::copy_tsv(&body(), &sel, 4, 2).unwrap();
    assert_eq!(tsv, "1\n2\n3\n4");
}

/// A copy over the cell limit is rejected, never truncated silently.
#[test]
fn an_oversized_copy_is_rejected() {
    let big: Vec<Vec<String>> = vec![vec!["x".into(); 100]; 700];
    let sel = Selection::select_all();
    let err = clipboard::copy_tsv(&big, &sel, 700, 100).unwrap_err();
    assert!(
        matches!(err, clipboard::TsvError::TooManyCells { cells, limit } if cells == 70000 && limit == MAX_TSV_CELLS)
    );
}

/// Paste parses a plain matrix, tolerates a trailing newline (spreadsheets
/// emit one) and unquotes symmetrically.
#[test]
fn paste_parses_a_plain_matrix_and_a_trailing_newline() {
    let matrix = clipboard::parse_tsv("1\talpha\n2\tbeta\n").unwrap();
    assert_eq!(matrix, vec![vec!["1", "alpha"], vec!["2", "beta"]]);
    assert_eq!(
        clipboard::parse_tsv("\"a\tb\"\tc\n\"x\"\"y\"\tz\n").unwrap(),
        vec![vec!["a\tb", "c"], vec!["x\"y", "z"]]
    );
}

/// Empty and ragged pastes are rejected with a precise error naming the
/// row.
#[test]
fn empty_and_ragged_pastes_are_rejected() {
    assert!(matches!(
        clipboard::parse_tsv(""),
        Err(clipboard::TsvError::Empty)
    ));
    let err = clipboard::parse_tsv("a\tb\nc").unwrap_err();
    assert!(
        matches!(err, clipboard::TsvError::NotRectangular { row, expected, found } if row == 1 && expected == 2 && found == 1)
    );
}

/// A paste over the cell limit is rejected before any target is computed.
#[test]
fn an_oversized_paste_is_rejected() {
    let text = "x\t".repeat(MAX_TSV_CELLS + 1);
    let err = clipboard::parse_tsv(&text).unwrap_err();
    assert!(matches!(err, clipboard::TsvError::TooManyCells { .. }));
}

/// A collapsed selection anchors the matrix at the cell; the matrix must
/// fit the loaded grid in both dimensions.
#[test]
fn a_collapsed_selection_anchors_the_paste() {
    let (rows, cols) = clipboard::paste_targets(&Selection::collapse(1, 1), 2, 1, 4, 2).unwrap();
    assert_eq!(rows, 1..3);
    assert_eq!(cols, 1..2);
    assert!(clipboard::paste_targets(&Selection::collapse(3, 1), 2, 1, 4, 2).is_err());
    assert!(clipboard::paste_targets(&Selection::collapse(1, 1), 2, 2, 4, 2).is_err());
}

/// A rectangle takes the matrix only when the shapes match exactly.
#[test]
fn a_rectangle_takes_an_exactly_shaped_matrix() {
    let sel = Selection::Rect {
        anchor: CellPos { row: 0, col: 0 },
        focus: CellPos { row: 1, col: 1 },
    };
    let (rows, cols) = clipboard::paste_targets(&sel, 2, 2, 4, 2).unwrap();
    assert_eq!(rows, 0..2);
    assert_eq!(cols, 0..2);
    assert!(clipboard::paste_targets(&sel, 3, 2, 4, 2).is_err());
}

/// A column selection is not a cell target — the rejection is precise,
/// not a panic.
#[test]
fn a_column_selection_rejects_a_paste() {
    assert!(clipboard::paste_targets(&Selection::select_column(0), 2, 2, 4, 2).is_err());
}

/// Select-all takes the matrix when it covers the whole loaded grid.
#[test]
fn select_all_takes_a_full_grid_matrix() {
    let (rows, cols) = clipboard::paste_targets(&Selection::All, 4, 2, 4, 2).unwrap();
    assert_eq!(rows, 0..4);
    assert_eq!(cols, 0..2);
    assert!(clipboard::paste_targets(&Selection::All, 3, 2, 4, 2).is_err());
}
