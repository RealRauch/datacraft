//! The CSV import surface (step 58.1, item M50): one external CSV file
//! becomes a table definition plus text rows — the `ImportFile` command
//! creates and fills the table over the ordinary write path.
//!
//! The L7 inference rules (crate docs of `datacraft-csv`, item L7) moved
//! here so the CSV driver and the import share one implementation; the
//! direction `csv → core` keeps the dependency graph acyclic. Cells stay
//! text at the value level — the caller coerces them through
//! `row_edit::row_values` with the precise per-field errors.

use std::path::Path;

use crate::schema::{FieldDef, FieldKind, NumericKind};

/// Maximum Short Text length of every CSV column (the L7 constant).
pub const MAX_TEXT_LEN: u16 = 255;

/// One parsed CSV source: the typed field definitions (header + inferred
/// kinds) and the raw text rows (without the header).
pub struct CsvTable {
    /// The inferred field definitions in column order.
    pub fields: Vec<FieldDef>,
    /// The data rows as raw text cells (no header).
    pub rows: Vec<Vec<String>>,
}

/// Reads and parses one CSV file: UTF-8 text with a leading BOM stripped,
/// ragged rows and invalid CSV fail with a locatable reason (LESSONS #4).
/// The source file is only ever read.
pub fn read_csv_file(path: &Path) -> Result<CsvTable, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| format!("{}: not valid UTF-8: {e}", path.display()))?;
    let text = match text.strip_prefix('\u{feff}') {
        Some(stripped) => stripped.to_owned(),
        None => text,
    };
    read_csv_text(&text)
}

/// Parses one CSV text into a [`CsvTable`]: the header row names the
/// fields, the L7 inference types them, every data row stays text.
pub fn read_csv_text(text: &str) -> Result<CsvTable, String> {
    let records = parse_records(text)?;
    let header = records
        .first()
        .ok_or_else(|| "the source has no header row".to_owned())?;
    let mask = quote_mask(text);
    let kinds = infer_field_kinds(&records, &mask);
    let fields = header
        .iter()
        .zip(kinds)
        .map(|(name, kind)| FieldDef::new(name.clone(), kind))
        .collect();
    Ok(CsvTable {
        fields,
        rows: records.into_iter().skip(1).collect(),
    })
}

/// Parses one CSV text into uniform records; ragged rows fail with a
/// locatable reason (the csv crate's `flexible(false)` makes a ragged row
/// a reader error, so every record carries the header's arity).
fn parse_records(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_reader(text.as_bytes());
    let mut records: Vec<Vec<String>> = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| format!("row {}: {e}", index + 1))?;
        records.push(record.into_iter().map(String::from).collect());
    }
    Ok(records)
}

/// Conservative, deterministic type inference per column (step 36.5,
/// item L7), decided from a full scan and stable across opens:
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
pub fn infer_field_kinds(records: &[Vec<String>], mask: &[Vec<bool>]) -> Vec<FieldKind> {
    let Some(header) = records.first() else {
        return Vec::new();
    };
    let mut kinds = vec![FieldKind::Text { max_len: MAX_TEXT_LEN }; header.len()];
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
            FieldKind::Text { max_len: MAX_TEXT_LEN }
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
pub fn quote_mask(text: &str) -> Vec<Vec<bool>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_integers_floats_and_dates_infer_their_kinds() {
        let text = "sku,price,day,label\n1,1.5,2026-01-02,bolt\n2,2.5,2026-01-03,nut\n";
        let table = read_csv_text(text).expect("the source parses");
        assert_eq!(table.fields.len(), 4);
        assert!(matches!(table.fields[0].kind, FieldKind::Number(NumericKind::LongInteger)));
        assert!(matches!(table.fields[1].kind, FieldKind::Number(NumericKind::Double)));
        assert!(matches!(table.fields[2].kind, FieldKind::DateTime));
        assert!(matches!(table.fields[3].kind, FieldKind::Text { .. }));
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn one_quoted_cell_pins_the_column_to_text() {
        let text = "sku,label\n1,\"plain bolt\"\n2,nut\n";
        let table = read_csv_text(text).expect("the source parses");
        assert!(matches!(table.fields[0].kind, FieldKind::Number(_)));
        assert!(matches!(table.fields[1].kind, FieldKind::Text { .. }));
    }

    #[test]
    fn ragged_rows_and_empty_sources_fail_precisely() {
        let ragged = read_csv_text("a,b\n1\n");
        assert!(ragged.is_err(), "a ragged row is a precise failure");
        assert!(read_csv_text("").is_err(), "an empty source has no header");
    }

    #[test]
    fn a_leading_bom_is_stripped() {
        let table = read_csv_text("\u{feff}a\n1\n").expect("the source parses");
        assert_eq!(table.fields[0].name, "a");
    }
}
