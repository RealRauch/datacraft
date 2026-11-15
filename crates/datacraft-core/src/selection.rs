//! Datasheet selection model (step 86.1, item M101).
//!
//! Pure, window-free state for the shared grid's multi-cell selection:
//! one collapsed cell, a rectangle over displayed rows and columns, whole
//! columns or everything loaded. The UI owns the per-tab value; the grid
//! renders and mutates it only through these functions, so the datasheet
//! and the query results select identically.
//!
//! # Survival rule (documented, M101 acceptance)
//!
//! A selection addresses **displayed positions** (the view order of the
//! loaded, filtered rows), never record identities:
//!
//! - **Scroll** never changes a selection — it is presentation state; the
//!   indices stay untouched and the painted window moves over them.
//! - **Sort/filter** changes which records the positions address. The
//!   stored value is clamped to the current row/column count on the next
//!   read ([`Selection::clamped`]): both corners move into the new
//!   bounds, a rectangle shrinks with them (a fully out-of-bounds
//!   rectangle meets the boundary as its intersection), a fully
//!   out-of-bounds cell moves onto the nearest boundary. Clamping is
//!   deterministic per input state, so a selection can never name a
//!   position that does not exist; 86.2's paste validates the matrix
//!   against the current values before any cell changes.
//! - **Schema refresh** (column count change) clamps columns the same
//!   way. `All` is bounds-free and covers whatever is loaded at read
//!   time.

/// One cell of a selection (display position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    /// Displayed row index.
    pub row: usize,
    /// Displayed column index.
    pub col: usize,
}

/// The selection of one grid: a single cell, a rectangle, whole columns
/// or everything loaded. `All` covers the loaded rows at read time and is
/// therefore not clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// One collapsed cell.
    Cell { row: usize, col: usize },
    /// A rectangle between the anchor and the focus corner (either
    /// corner may be the smaller one; the ranges normalize).
    Rect { anchor: CellPos, focus: CellPos },
    /// Whole columns between the anchor and the focus column.
    Columns { anchor: usize, focus: usize },
    /// Everything loaded (Ctrl+A).
    All,
}

impl Selection {
    /// A collapsed selection on one cell.
    pub fn collapse(row: usize, col: usize) -> Self {
        Self::Cell { row, col }
    }

    /// Everything loaded (Ctrl+A).
    pub fn select_all() -> Self {
        Self::All
    }

    /// Whole columns on `col` (the header's "Select Column" affordance).
    pub fn select_column(col: usize) -> Self {
        Self::Columns {
            anchor: col,
            focus: col,
        }
    }

    /// Extends (or collapses) a cell or rectangle selection to the cell
    /// at `(row, col)`: a shift click or shift-arrow target. The anchor
    /// survives; extending from a column selection anchors at its anchor
    /// column's first row; extending from `All` anchors at the origin.
    pub fn extend_to_cell(self, row: usize, col: usize) -> Self {
        let anchor = self.anchor_cell();
        Self::Rect { anchor, focus: CellPos { row, col } }
    }

    /// Extends a column selection to `col` (shift click on a header);
    /// the anchor column survives and the span covers both directions.
    /// Extending a non-column selection starts a fresh column selection
    /// on `col` with itself as the anchor.
    pub fn extend_columns(self, col: usize) -> Self {
        let anchor = match self {
            Self::Columns { anchor, .. } => anchor,
            _ => col,
        };
        Self::Columns { anchor, focus: col }
    }

    /// The anchor cell the next extension extends from.
    pub fn anchor_cell(&self) -> CellPos {
        match *self {
            Self::Cell { row, col } | Self::Rect { anchor: CellPos { row, col }, .. } => {
                CellPos { row, col }
            }
            Self::Columns { anchor, .. } => CellPos { row: 0, col: anchor },
            Self::All => CellPos { row: 0, col: 0 },
        }
    }

    /// The normalized, bounds-clamped row span of the selection:
    /// `[first, last_exclusive)`. `All` and column selections cover every
    /// loaded row.
    pub fn row_range(&self, rows: usize) -> std::ops::Range<usize> {
        match *self {
            Self::Cell { row, .. } => row.min(rows.saturating_sub(1))..row.min(rows.saturating_sub(1)) + 1,
            Self::Rect { anchor, focus } => {
                let (first, last) = ordered(anchor.row, focus.row);
                first.min(rows.saturating_sub(1))..(last.min(rows.saturating_sub(1)) + 1).min(rows)
            }
            Self::Columns { .. } | Self::All => 0..rows,
        }
    }

    /// The normalized, bounds-clamped column span of the selection:
    /// `[first, last_exclusive)`. `All` covers every column.
    pub fn col_range(&self, cols: usize) -> std::ops::Range<usize> {
        match *self {
            Self::Cell { col, .. } => col.min(cols.saturating_sub(1))..col.min(cols.saturating_sub(1)) + 1,
            Self::Rect { anchor, focus } => {
                let (first, last) = ordered(anchor.col, focus.col);
                first.min(cols.saturating_sub(1))..(last.min(cols.saturating_sub(1)) + 1).min(cols)
            }
            Self::Columns { anchor, focus } => {
                let (first, last) = ordered(anchor, focus);
                first.min(cols.saturating_sub(1))..(last.min(cols.saturating_sub(1)) + 1).min(cols)
            }
            Self::All => 0..cols,
        }
    }

    /// Whether the cell at `(row, col)` is part of the selection, with
    /// the loaded bounds supplied by the caller.
    pub fn contains(&self, row: usize, col: usize, rows: usize, cols: usize) -> bool {
        self.row_range(rows).contains(&row) && self.col_range(cols).contains(&col)
    }

    /// The survival rule (module docs): clamps every addressed corner
    /// into the loaded bounds. Rectangles shrink with their corners; a
    /// fully out-of-bounds selection clamps to the nearest in-bounds
    /// cell; `All` is bounds-free and passes through unchanged.
    pub fn clamped(self, rows: usize, cols: usize) -> Self {
        let clamp_cell = |cell: CellPos| CellPos {
            row: cell.row.min(rows.saturating_sub(1)),
            col: cell.col.min(cols.saturating_sub(1)),
        };
        match self {
            Self::Cell { row, col } => {
                let cell = clamp_cell(CellPos { row, col });
                Self::Cell {
                    row: cell.row,
                    col: cell.col,
                }
            }
            Self::Rect { anchor, focus } => {
                let anchor = clamp_cell(anchor);
                let focus = clamp_cell(focus);
                if anchor == focus {
                    Self::Cell {
                        row: anchor.row,
                        col: anchor.col,
                    }
                } else {
                    Self::Rect { anchor, focus }
                }
            }
            Self::Columns { anchor, focus } => {
                let anchor = anchor.min(cols.saturating_sub(1));
                Self::Columns {
                    anchor,
                    focus: focus.min(cols.saturating_sub(1)),
                }
            }
            Self::All => Self::All,
        }
    }
}

/// Ordered pair `(smaller, larger)` of two indices.
fn ordered(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A collapsed cell's ranges are exactly one wide and in bounds.
    #[test]
    fn a_collapsed_cell_covers_exactly_itself() {
        let sel = Selection::collapse(4, 2);
        assert_eq!(sel.row_range(10), 4..5);
        assert_eq!(sel.col_range(5), 2..3);
        assert!(sel.contains(4, 2, 10, 5));
        assert!(!sel.contains(4, 3, 10, 5));
    }

    /// Extending a column selection to a cell starts a rectangle from the
    /// anchor column's first row.
    #[test]
    fn extending_a_column_selection_to_a_cell_starts_at_the_anchor_column() {
        let sel = Selection::select_column(3).extend_to_cell(4, 1);
        assert_eq!(sel, Selection::Rect {
            anchor: CellPos { row: 0, col: 3 },
            focus: CellPos { row: 4, col: 1 },
        });
        assert_eq!(sel.row_range(10), 0..5);
        assert_eq!(sel.col_range(5), 1..4);
    }
}
