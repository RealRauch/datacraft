//! Index-maintenance internals (step 49.2, item M63): the refusal and
//! bookkeeping branches that the acceptance path does not reach.

use super::*;
use crate::jet::index_pages::{PhysicalIndex, RowPointer};

/// The embedded engine template (catalog indexes at pages 7/8).
const FRESH: &[u8] = include_bytes!("../fresh_template.mdb");
const CATALOG: u32 = 2;

fn writer() -> Jet4Writer {
    Jet4Writer::new(FRESH.to_vec()).expect("template opens")
}

#[test]
fn row_slots_beyond_one_byte_are_refused() {
    let error = row_pointer(5, 256).expect_err("slot 256 does not fit");
    assert!(
        matches!(error, StorageError::Unsupported { .. }),
        "{error:?}"
    );
    assert_eq!(
        row_pointer(5, 255).expect("fits"),
        RowPointer { page: 5, row: 255 }
    );
}

#[test]
fn distinct_keys_ignore_the_row_pointer() {
    let entry = |key: u8, row: u8| vec![0x7F, key, 0x00, 0x00, 0x00, 0x11, row];
    let entries = vec![entry(1, 0), entry(1, 1), entry(2, 0)];
    assert_eq!(distinct_keys(&entries), 2);
    assert_eq!(distinct_keys(&[]), 0);
}

#[test]
fn unsupported_key_types_answer_a_precise_error() {
    let def = JetTableDef::decode(FRESH, CATALOG).expect("catalog");
    let index_of = |name: &str| {
        def.columns
            .iter()
            .find(|column| column.name == name)
            .expect("column")
            .column_id
    };
    let mut values = vec![StorageValue::Null; def.columns.len()];
    let pointer = RowPointer { page: 1, row: 0 };

    // A memo column cannot be keyed.
    let memo = index_of("Database");
    let position = def
        .columns
        .iter()
        .position(|column| column.name == "Database")
        .expect("memo");
    values[position] = StorageValue::Text("not indexable".into());
    let index = PhysicalIndex {
        columns: vec![memo],
        root_page: 7,
        stats_offset: 0,
    };
    let error = Jet4Writer::index_entry(&def, &index, &values, pointer)
        .expect_err("memo keys are not encodable");
    assert!(
        matches!(error, StorageError::Unsupported { .. }),
        "{error:?}"
    );
    values[position] = StorageValue::Null;

    // An index that references a column the definition does not have.
    let missing = PhysicalIndex {
        columns: vec![u16::MAX],
        root_page: 7,
        stats_offset: 0,
    };
    assert!(matches!(
        Jet4Writer::index_entry(&def, &missing, &values, pointer),
        Err(StorageError::Corrupt { .. })
    ));

    // A value list shorter than the definition.
    let id = index_of("Id");
    let short = PhysicalIndex {
        columns: vec![id],
        root_page: 8,
        stats_offset: 0,
    };
    assert!(matches!(
        Jet4Writer::index_entry(&def, &short, &[], pointer),
        Err(StorageError::Corrupt { .. })
    ));
}

#[test]
fn leaf_edits_refuse_foreign_or_missing_roots() {
    let mut writer = writer();
    // Page 3 is a table definition, not a leaf.
    let not_a_leaf = PhysicalIndex {
        columns: vec![0],
        root_page: 3,
        stats_offset: 0,
    };
    let error = writer
        .edit_index_leaf(CATALOG, &not_a_leaf, |_| Ok(()))
        .expect_err("TDEF page is not a leaf");
    assert!(
        matches!(error, StorageError::Unsupported { .. }),
        "{error:?}"
    );

    // Leaf 10 belongs to MSysACEs, not to the catalog.
    let foreign = PhysicalIndex {
        columns: vec![0],
        root_page: 10,
        stats_offset: 0,
    };
    let error = writer
        .edit_index_leaf(CATALOG, &foreign, |_| Ok(()))
        .expect_err("foreign leaf");
    assert!(matches!(error, StorageError::Corrupt { .. }), "{error:?}");

    // A root beyond the file.
    let beyond = PhysicalIndex {
        columns: vec![0],
        root_page: 9_999,
        stats_offset: 0,
    };
    assert!(
        writer
            .edit_index_leaf(CATALOG, &beyond, |_| Ok(()))
            .is_err()
    );
}

#[test]
fn tables_without_physical_indexes_skip_every_hook() {
    let mut writer = writer();
    let mut def = JetTableDef::decode(FRESH, CATALOG).expect("catalog");
    def.real_index_count = 0;
    def.index_section.clear();
    writer
        .maintain_indexes(CATALOG, &def, &[], 1, 0)
        .expect("no indexes, nothing to do");
    writer
        .remove_index_entries(CATALOG, &def, 1, 0)
        .expect("no indexes, nothing to remove");
    writer
        .replace_index_entries(CATALOG, &def, 1, 0, &[])
        .expect("no indexes, nothing to replace");
}

/// Every supported key type maps to its observed sort-ordered form; the
/// narrowing casts mirror the row encoder.
#[test]
fn every_scalar_type_maps_to_its_key_part() {
    use crate::jet::index_pages::KeyPart;
    let column = |data_type: JetType| JetColumn::new("k", data_type, 0x03, 0, 0, 8);

    let bit = StorageValue::Bit(true);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Bool), &bit).expect("bool"),
        KeyPart::Boolean(true)
    );
    let byte = StorageValue::Int(200);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Byte), &byte).expect("byte"),
        KeyPart::Byte(200)
    );
    let int = StorageValue::Int(-1);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Int), &int).expect("int"),
        KeyPart::SmallInteger(-1)
    );
    let long = StorageValue::Int(7);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::LongInt), &long).expect("long"),
        KeyPart::Integer(7)
    );
    let currency = StorageValue::Currency(15_000);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Currency), &currency).expect("currency"),
        KeyPart::Currency(15_000)
    );
    let single = StorageValue::Float(1.5);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Float), &single).expect("single"),
        KeyPart::Single(1.5)
    );
    let double = StorageValue::Float(1.5);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Double), &double).expect("double"),
        KeyPart::Double(1.5)
    );
    let date = StorageValue::DateTime(43_831.0);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::DateTime), &date).expect("date"),
        KeyPart::DateTime(43_831.0)
    );
    let guid_value = StorageValue::Guid([7u8; 16]);
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Guid), &guid_value).expect("guid"),
        KeyPart::Guid([7u8; 16])
    );
    let text = StorageValue::Text("x".into());
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::Text), &text).expect("text"),
        KeyPart::Text("x")
    );
    let null = StorageValue::Null;
    assert_eq!(
        Jet4Writer::key_part(&column(JetType::LongInt), &null).expect("null"),
        KeyPart::Null
    );
    let memo = StorageValue::Text("memo".into());
    assert!(matches!(
        Jet4Writer::key_part(&column(JetType::Memo), &memo),
        Err(StorageError::Unsupported { .. })
    ));
}
