//! Find and replace over the visible rows (phase 101, step 101.3; item
//! M132; roadmap §16 stage 1).
//!
//! The core owns the match model: which cells of the datasheet's current
//! display order carry the needle and what the replaced text is. Only
//! text-like columns take part (short text and memo) — a replacement is a
//! text edit, and coercing a rendered string back into a number or a date
//! would guess. The UI stages every match into the edit buffer, so a
//! Replace All is one pending batch and one atomic write.

use datacraft_core::schema::{FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::table_view::{FindRequest, find_matches};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            datacraft_core::schema::FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            datacraft_core::schema::FieldDef::new("note", FieldKind::Memo),
            datacraft_core::schema::FieldDef::new(
                "qty",
                FieldKind::Number(NumericKind::LongInteger),
            ),
        ],
    )
}

fn rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Text("Ada Lovelace".into()),
            FieldValue::Text("pioneer of ADA".into()),
            FieldValue::Integer(1),
        ],
        vec![
            FieldValue::Text("Grace Hopper".into()),
            FieldValue::Null,
            FieldValue::Integer(2),
        ],
        vec![
            FieldValue::Null,
            FieldValue::Text("ada again".into()),
            FieldValue::Integer(3),
        ],
    ]
}

fn request(text: &str) -> FindRequest {
    FindRequest {
        text: text.into(),
        replace_with: "Grace".into(),
        column: None,
        match_case: false,
        whole_field: false,
    }
}

#[test]
fn the_search_is_case_insensitive_by_default_and_skips_other_families() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let matches = find_matches(&table, &rows, &order, &request("ada")).expect("runs");
    let cells: Vec<(usize, usize, &str)> = matches
        .iter()
        .map(|found| (found.row, found.column, found.cell.as_str()))
        .collect();
    assert_eq!(
        cells,
        vec![
            (0, 0, "Ada Lovelace"),
            (0, 1, "pioneer of ADA"),
            (2, 1, "ada again"),
        ],
        "text and memo columns match, numbers do not, blanks never do"
    );
}

#[test]
fn the_replacement_text_replaces_every_occurrence_in_the_cell() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let matches = find_matches(&table, &rows, &order, &request("ada")).expect("runs");
    assert_eq!(matches[0].replaced, "Grace Lovelace");
    assert_eq!(matches[1].replaced, "pioneer of Grace");
}

#[test]
fn match_case_and_whole_field_narrow_the_matches() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let mut exact = request("Ada");
    exact.match_case = true;
    let matches = find_matches(&table, &rows, &order, &exact).expect("runs");
    assert_eq!(matches.len(), 1, "{matches:?}");
    assert_eq!(matches[0].cell, "Ada Lovelace");

    let mut whole = request("ada again");
    whole.whole_field = true;
    let matches = find_matches(&table, &rows, &order, &whole).expect("runs");
    assert_eq!(matches.len(), 1, "{matches:?}");
    assert_eq!(matches[0].column, 1);
}

#[test]
fn a_column_scope_limits_the_search_and_must_be_text() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let mut scoped = request("ada");
    scoped.column = Some("note".into());
    let matches = find_matches(&table, &rows, &order, &scoped).expect("runs");
    assert_eq!(matches.len(), 2, "{matches:?}");

    let mut numeric = request("1");
    numeric.column = Some("qty".into());
    let error = find_matches(&table, &rows, &order, &numeric).expect_err("refused");
    assert!(error.contains("text"), "{error}");
}

#[test]
fn only_the_visible_rows_take_part() {
    // The display order is the filter's survivor list: a filtered-out row
    // never matches, so Replace All cannot touch it.
    let table = customers();
    let rows = rows();
    let order = vec![1usize];
    let matches = find_matches(&table, &rows, &order, &request("ada")).expect("runs");
    assert!(matches.is_empty(), "{matches:?}");
}

#[test]
fn an_empty_needle_finds_nothing() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let matches = find_matches(&table, &rows, &order, &request("   ")).expect("runs");
    assert!(matches.is_empty(), "a blank needle is not a match");
}

#[test]
fn a_needle_longer_than_the_cell_finds_nothing() {
    let table = customers();
    let rows = rows();
    let order: Vec<usize> = (0..rows.len()).collect();
    let matches = find_matches(
        &table,
        &rows,
        &order,
        &request("Ada Lovelace and then some more text"),
    )
    .expect("runs");
    assert!(matches.is_empty(), "{matches:?}");
}
