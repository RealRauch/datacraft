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
