//! Conservative column type inference (step 36.5, item L7) — split from
//! the crate root (step 36.7) to keep every file under the 500-line size
//! gate. The rule, the strict spellings and the RFC-4180 quote mask live
//! here; the crate root owns the port surface.

use datacraft_core::schema::{FieldKind, NumericKind};

/// Maximum Short Text length of every CSV column.
pub(super) const MAX_LEN: u16 = 255;

/// Conservative, deterministic type inference per column (step 36.5,
/// item L7), decided at open from a full scan and stable across opens:
///
/// - every unquoted, non-empty cell of a column parses as a strict
///   integer → `LongInteger`;
/// - otherwise every such cell parses as a strict float → `Double`;
/// - otherwise every such cell is an ISO date (`YYYY-MM-DD`) →
///   `DateTime`;
/// - anything else stays Short Text.
///
/// Quoting is the user's escape hatch (CSV semantics): **any** quoted
/// cell in a column pins the whole column to Short Text — a column-level
/// schema can never honor per-cell quoting, so ambiguity never guesses.
/// Empty cells don't vote. Cells stay text at the value level; only the
/// schema kind is inferred.
pub(super) fn infer_field_kinds(records: &[Vec<String>], mask: &[Vec<bool>]) -> Vec<FieldKind> {
    let Some(header) = records.first() else {
        return Vec::new();
    };
    let mut kinds = vec![FieldKind::Text { max_len: MAX_LEN }; header.len()];
    for (col, kind) in kinds.iter_mut().enumerate() {
        let mut any = false;
        let mut all_int = true;
        let mut all_float = true;
        let mut all_date = true;
        let mut any_quoted = false;
        for (row, cells) in records.iter().enumerate().skip(1) {
            let Some(cell) = cells.get(col) else {
                any = false;
                break;
            };
            if mask
                .get(row)
                .and_then(|cells| cells.get(col))
                .copied()
                .unwrap_or(false)
            {
                any_quoted = true;
                continue;
            }
            if cell.is_empty() {
                continue;
            }
            any = true;
            all_int &= is_strict_int(cell);
            all_float &= is_strict_float(cell);
            all_date &= is_iso_date(cell);
        }
        if !any || any_quoted {
            continue;
        }
        *kind = if all_int {
            FieldKind::Number(NumericKind::LongInteger)
        } else if all_float {
            FieldKind::Number(NumericKind::Double)
        } else if all_date {
            FieldKind::DateTime
        } else {
            FieldKind::Text { max_len: MAX_LEN }
        };
    }
    kinds
}

/// A strict integer: optional minus, digits only (no sign-plus, no
/// whitespace — the conservative side).
fn is_strict_int(cell: &str) -> bool {
    let digits = cell.strip_prefix('-').unwrap_or(cell);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// A strict float: parses as f64 without infinities or NaN (the accepted
/// spellings of those contain letters; digits, sign, dot and exponent
/// notation stay allowed).
fn is_strict_float(cell: &str) -> bool {
    cell.parse::<f64>().is_ok()
        && cell
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.' | b'e' | b'E'))
}

/// A strict ISO date: `YYYY-MM-DD` with a plausible month and day.
fn is_iso_date(cell: &str) -> bool {
    let bytes = cell.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (y, m, d) = (&bytes[0..4], &bytes[5..7], &bytes[8..10]);
    let digits = |slice: &[u8]| slice.iter().all(|b| b.is_ascii_digit());
    let number = |slice: &[u8]| -> u32 {
        slice
            .iter()
            .fold(0u32, |acc, b| acc * 10 + u32::from(b - b'0'))
    };
    digits(y)
        && digits(m)
        && digits(d)
        && (1..=12).contains(&number(m))
        && (1..=31).contains(&number(d))
}

/// Marks which parsed cells were quoted in the raw text (L7): a small
/// RFC-4180 state machine over the same bytes the CSV crate parsed —
/// quoted fields, escaped `""` quotes and embedded commas/newlines stay
/// inside the quotes. Only the *flag* is consumed (for inference); the
/// parsed records remain the csv crate's single source of truth. A mask
/// that disagrees with the records degrades to all-unquoted.
pub(super) fn quote_mask(text: &str) -> Vec<Vec<bool>> {
    let mut rows: Vec<Vec<bool>> = Vec::new();
    let mut row: Vec<bool> = Vec::new();
    let mut cell_quoted = false;
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
        } else {
            match c {
                '"' => {
                    in_quotes = true;
                    cell_quoted = true;
                }
                ',' => {
                    row.push(cell_quoted);
                    cell_quoted = false;
                }
                '\n' => {
                    row.push(cell_quoted);
                    rows.push(std::mem::take(&mut row));
                    cell_quoted = false;
                }
                _ => {}
            }
        }
    }
    if !row.is_empty() || rows.is_empty() {
        row.push(cell_quoted);
        rows.push(row);
    }
    rows
}
