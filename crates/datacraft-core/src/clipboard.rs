//! Clipboard TSV vocabulary (step 86.2, item M101).
//!
//! Pure, window-free text conversion for the shared grid's clipboard:
//! copy builds a spreadsheet-compatible TSV over the selection, paste
//! parses such a text back into a matrix, and [`paste_targets`] maps the
//! matrix onto the selection — rejecting shape and size mismatches
//! **before any cell changes**. The UI owns the system clipboard itself;
//! these functions only convert and validate.

/// Cell budget of one clipboard operation, in both directions. The bound
/// keeps a runaway selection (or a hostile clipboard text) from building
/// unbounded matrices; a larger transfer is rejected with
/// [`TsvError::TooManyCells`], never truncated silently.
pub const MAX_TSV_CELLS: usize = 65536;

/// The clipboard TSV errors — precise and locatable (AGENTS §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsvError {
    /// The selection (copy) or the matrix (paste) exceeds
    /// [`MAX_TSV_CELLS`].
    TooManyCells { cells: usize, limit: usize },
    /// A pasted row has a different column count than the first one.
    NotRectangular {
        /// The zero-based row that diverged.
        row: usize,
        /// The first row's column count.
        expected: usize,
        /// The diverging row's column count.
        found: usize,
    },
    /// The clipboard text carries no matrix (empty or whitespace only).
    Empty,
}

impl std::fmt::Display for TsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCells { cells, limit } => {
                write!(f, "{cells} cells exceed the clipboard limit of {limit}")
            }
            Self::NotRectangular {
                row,
                expected,
                found,
            } => write!(
                f,
                "row {} has {found} columns, the first row has {expected}",
                row + 1
            ),
            Self::Empty => write!(f, "the clipboard holds no table text"),
        }
    }
}

/// The paste fit errors — the matrix did not map onto the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteError {
    /// The selection is a rectangle but the matrix has different
    /// dimensions.
    ShapeMismatch {
        /// The selection's (rows, columns).
        expected: (usize, usize),
        /// The matrix's (rows, columns).
        found: (usize, usize),
    },
    /// A collapsed-cell paste does not fit the loaded grid.
    DoesNotFit {
        /// The matrix's (rows, columns).
        found: (usize, usize),
        /// The loaded (rows, columns) from the anchor cell on.
        available: (usize, usize),
    },
    /// A column selection addresses whole columns, not cell targets.
    ColumnsNotACellTarget,
}

impl std::fmt::Display for PasteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShapeMismatch { expected, found } => write!(
                f,
                "the pasted matrix is {}x{} but the selection is {}x{}",
                found.0, found.1, expected.0, expected.1
            ),
            Self::DoesNotFit { found, available } => write!(
                f,
                "the pasted matrix is {}x{} but only {}x{} fit from the selected cell on",
                found.0, found.1, available.0, available.1
            ),
            Self::ColumnsNotACellTarget => write!(
                f,
                "a column selection cannot take pasted cells — select the target cell first"
            ),
        }
    }
}

/// Builds the TSV text of the selection over the displayed body (the
/// value-label strings the grid paints). Cells join with tabs, rows with
/// newlines; cells containing tabs, newlines or quotes are Excel-style
/// quoted so a spreadsheet round-trips the text (the trailing newline a
/// spreadsheet adds is the paste path's tolerance, not the copy's
/// output).
pub fn copy_tsv(
    body: &[Vec<String>],
    selection: &crate::selection::Selection,
    rows: usize,
    cols: usize,
) -> Result<String, TsvError> {
    let row_span = selection.row_range(rows);
    let col_span = selection.col_range(cols);
    let cells = row_span.len() * col_span.len();
    if cells > MAX_TSV_CELLS {
        return Err(TsvError::TooManyCells {
            cells,
            limit: MAX_TSV_CELLS,
        });
    }
    let mut out = String::new();
    let last_row = row_span.end.saturating_sub(1);
    for row in row_span {
        let mut first = true;
        for col in col_span.clone() {
            if !first {
                out.push('\t');
            }
            first = false;
            push_quoted(
                &mut out,
                body.get(row)
                    .and_then(|cells| cells.get(col))
                    .map(String::as_str)
                    .unwrap_or_default(),
            );
        }
        if row < last_row {
            out.push('\n');
        }
    }
    Ok(out)
}

/// Parses a pasted TSV text into a rectangular string matrix. A trailing
/// newline is tolerated (spreadsheets emit one); quoted cells may span
/// newlines (the symmetric reverse of [`copy_tsv`]'s quoting); ragged
/// rows and over-budget matrices are rejected with the precise error.
pub fn parse_tsv(text: &str) -> Result<Vec<Vec<String>>, TsvError> {
    if text.trim().is_empty() {
        return Err(TsvError::Empty);
    }
    let mut matrix: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = vec![String::new()];
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        row.last_mut().expect("row starts with one cell").push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                }
                other => row
                    .last_mut()
                    .expect("row starts with one cell")
                    .push(other),
            }
            continue;
        }
        match ch {
            '\t' => row.push(String::new()),
            '\n' => {
                matrix.push(std::mem::replace(&mut row, vec![String::new()]));
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                matrix.push(std::mem::replace(&mut row, vec![String::new()]));
            }
            '"' if row.last().is_some_and(String::is_empty) => in_quotes = true,
            other => row
                .last_mut()
                .expect("row starts with one cell")
                .push(other),
        }
    }
    // The final row: dropped when the text ended with a row separator
    // (row is freshly empty) — that is the trailing-newline tolerance.
    if row.len() > 1 || !row[0].is_empty() {
        matrix.push(row);
    }
    if matrix.is_empty() {
        return Err(TsvError::Empty);
    }
    let width = matrix[0].len();
    for (row, cells) in matrix.iter().enumerate().skip(1) {
        if cells.len() != width {
            return Err(TsvError::NotRectangular {
                row,
                expected: width,
                found: cells.len(),
            });
        }
    }
    let total = matrix.len() * width;
    if total > MAX_TSV_CELLS {
        return Err(TsvError::TooManyCells {
            cells: total,
            limit: MAX_TSV_CELLS,
        });
    }
    Ok(matrix)
}

/// Maps a parsed matrix onto the selection: a rectangle takes an exactly
/// shaped matrix, a collapsed cell anchors the matrix (which must fit
/// the loaded grid), a column selection rejects, `All` takes a full-grid
/// matrix. Returns the target row and column spans.
pub fn paste_targets(
    selection: &crate::selection::Selection,
    matrix_rows: usize,
    matrix_cols: usize,
    rows: usize,
    cols: usize,
) -> Result<(std::ops::Range<usize>, std::ops::Range<usize>), PasteError> {
    match *selection {
        crate::selection::Selection::Cell { row, col } => {
            let fit_rows = rows.saturating_sub(row);
            let fit_cols = cols.saturating_sub(col);
            if matrix_rows > fit_rows || matrix_cols > fit_cols {
                return Err(PasteError::DoesNotFit {
                    found: (matrix_rows, matrix_cols),
                    available: (fit_rows, fit_cols),
                });
            }
            Ok((row..row + matrix_rows, col..col + matrix_cols))
        }
        crate::selection::Selection::Rect { anchor, focus } => {
            let row_span = anchor.row.min(focus.row)..anchor.row.max(focus.row) + 1;
            let col_span = anchor.col.min(focus.col)..anchor.col.max(focus.col) + 1;
            if row_span.len() != matrix_rows || col_span.len() != matrix_cols {
                return Err(PasteError::ShapeMismatch {
                    expected: (row_span.len(), col_span.len()),
                    found: (matrix_rows, matrix_cols),
                });
            }
            Ok((row_span, col_span))
        }
        crate::selection::Selection::Columns { .. } => Err(PasteError::ColumnsNotACellTarget),
        crate::selection::Selection::All => {
            if matrix_rows != rows || matrix_cols != cols {
                return Err(PasteError::ShapeMismatch {
                    expected: (rows, cols),
                    found: (matrix_rows, matrix_cols),
                });
            }
            Ok((0..rows, 0..cols))
        }
    }
}

/// Excel-style quoting: a cell containing a tab, newline or quote is
/// wrapped in quotes with the quotes doubled.
fn push_quoted(out: &mut String, cell: &str) {
    if cell.contains(['\t', '\n', '\r', '"']) {
        out.push('"');
        for ch in cell.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
    } else {
        out.push_str(cell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser accepts CR-LF rows (the Windows spreadsheet form).
    #[test]
    fn paste_accepts_crlf_rows() {
        let matrix = parse_tsv("a\tb\r\nc\td").unwrap();
        assert_eq!(matrix, vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    /// An unterminated quote degrades to its literal text instead of
    /// panicking; text after a closing quote is literal.
    #[test]
    fn unterminated_quotes_degrade_to_literals() {
        assert_eq!(parse_tsv("\"open").unwrap(), vec![vec!["open".to_owned()]]);
        assert_eq!(
            parse_tsv("\"a\"b\tc").unwrap(),
            vec![vec!["ab".to_owned(), "c".to_owned()]]
        );
    }
}
