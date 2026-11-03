use super::*;
use crate::jet::encode::{build_table_def, encode_row};

fn memo_table() -> JetTableDef {
    build_table_def(
        0x4E,
        0,
        &[
            JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("note", JetType::Memo, 0x02, 1, 0, 0),
        ],
        0,
        0,
    )
}

#[test]
fn inline_long_values_are_rewritable_and_external_ones_are_not() {
    let def = memo_table();
    let inline = encode_row(
        &[StorageValue::Int(1), StorageValue::Text("hello".into())],
        &def,
    )
    .expect("encode inline memo");
    assert!(!has_external_long_value(&inline, &def).expect("inline row parses"));

    let null_memo =
        encode_row(&[StorageValue::Int(2), StorageValue::Null], &def).expect("encode null memo");
    assert!(!has_external_long_value(&null_memo, &def).expect("null memo parses"));

    // The memo payload starts behind num_cols (2) + the fixed area (4).
    let mut external = inline.clone();
    external[6..10].copy_from_slice(&0x4000_0000u32.to_le_bytes());
    assert!(has_external_long_value(&external, &def).expect("external row parses"));
}

#[test]
fn all_fixed_rows_decode_without_a_variable_tail() {
    // Real Access 365 writes rows of tables without variable columns as
    // [num_cols][fixed data][null mask] — no `eod`, no var table, no
    // `var_len` (observed on the self-created programmability fixture).
    // The tail-less layout only trips the tail parse when the last fixed
    // bytes read as a large u16 (e.g. a DateTime's exponent bytes), so the
    // decoder never used these rows at all. Fixed decoding needs neither
    // the tail nor the layout: values sit at their TDEF offsets and the
    // mask always ends the row.
    let def = build_table_def(
        0x4E,
        0,
        &[
            JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("granted", JetType::DateTime, 0x03, 1, 4, 8),
        ],
        0,
        0,
    );
    let mut row = Vec::new();
    row.extend_from_slice(&2u16.to_le_bytes());
    row.extend_from_slice(&7i32.to_le_bytes());
    row.extend_from_slice(&45000.5f64.to_le_bytes());
    row.push(0x03);
    let values = decode_row(&row, &def, &[], 4096).expect("tail-less fixed row decodes");
    assert_eq!(values[0], StorageValue::Int(7));
    match &values[1] {
        StorageValue::DateTime(_) => {}
        other => panic!("expected DateTime, got {other:?}"),
    }
}
