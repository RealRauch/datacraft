//! Table-creation and row-insertion contract tests (step 10.2, item H9).
//!
//! The catalog recipe (MSysObjects row for a new table) and the free-map
//! polarity are black-box verified against the fixture — see
//! `docs/FORMAT-JET.md` "Usage maps (writer)" and "Catalog insert recipe".

use datacraft_storage::jet::encode::{encode_row, encode_tdef};
use datacraft_storage::jet::usage_map::UsageMap;
use datacraft_storage::jet::{Jet4TableReader, Jet4Writer, JetColumn, JetTableDef, JetType};
use datacraft_storage::{StorageValue, jet};

mod support;

#[test]
fn row_encoder_round_trips_every_type() {
    let def = JetTableDef {
        table_type: 0x4E,
        num_rows: 0,
        used_pages: 0,
        free_pages: 0,
        columns: vec![
            JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("Price", JetType::Currency, 0x03, 1, 4, 8),
            JetColumn::new("Ratio", JetType::Double, 0x03, 2, 12, 8),
            JetColumn::new("When", JetType::DateTime, 0x03, 3, 20, 8),
            JetColumn::new("Active", JetType::Bool, 0x01, 4, 0, 0),
            JetColumn::new("Name", JetType::Text, 0x02, 5, 0, 20),
            JetColumn::new("Blob", JetType::Binary, 0x02, 6, 0, 255),
            JetColumn::new("Notes", JetType::Memo, 0x02, 7, 0, 0),
        ],
    };
    // var_index assignment mirrors the descriptor derivation for new tables.
    for (i, col) in def.columns.iter_mut().enumerate() {
        if !col.is_fixed() {
            col.var_index = Some(i as u16);
        }
    }
    let values = vec![
        StorageValue::Int(42),
        StorageValue::Currency(-123456),
        StorageValue::Float(2.5),
        StorageValue::DateTime(40000.5),
        StorageValue::Bit(true),
        StorageValue::Text("hello".into()),
        StorageValue::Binary(vec![1, 2, 3]),
        StorageValue::Binary(b"inline memo".to_vec()),
    ];
    let row = encode_row(&values, &def).expect("encodes");
    let decoded = jet::decode_row(&row, &def, &[], 4096).expect("decodes");
    assert_eq!(decoded, values);
}

#[test]
fn row_encoder_handles_nulls_and_empty_slots() {
    let mut def = JetTableDef {
        table_type: 0x4E,
        num_rows: 0,
        used_pages: 0,
        free_pages: 0,
        columns: vec![
            JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 20),
            JetColumn::new("Flag", JetType::Bool, 0x03, 2, 0, 0),
        ],
    };
    def.columns[1].var_index = Some(0);
    let values = vec![StorageValue::Int(7), StorageValue::Null, StorageValue::Bit(false)];
    let row = encode_row(&values, &def).expect("encodes");
    let decoded = jet::decode_row(&row, &def, &[], 4096).expect("decodes");
    assert_eq!(decoded, values);

    // Empty text is stored as an empty slot and reads back as "".
    let values = vec![StorageValue::Int(7), StorageValue::Text(String::new()), StorageValue::Bit(true)];
    let row = encode_row(&values, &def).expect("encodes");
    let decoded = jet::decode_row(&row, &def, &[], 4096).expect("decodes");
    assert_eq!(decoded[1], StorageValue::Text(String::new()));
}

#[test]
fn tdef_encoder_round_trips_through_the_decoder() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 20),
        JetColumn::new("Active", JetType::Bool, 0x01, 2, 4, 0),
    ];
    let def = JetTableDef::from_columns(0x4E, 7, &columns, 0x0003_1200, 0x0003_1201);
    let block = encode_tdef(&def);
    // Write the block into a synthetic file and decode it back.
    let mut file = support::file_shell(2);
    let base = support::PAGE;
    file[base] = jet::PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    let len = block.len() as u32;
    file[base + 8..base + 12].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
    let decoded = JetTableDef::decode(&file, 1).expect("decodes");
    assert_eq!(decoded.table_type, 0x4E);
    assert_eq!(decoded.num_rows, 7);
    assert_eq!(decoded.used_pages, 0x0003_1200);
    assert_eq!(decoded.free_pages, 0x0003_1201);
    assert_eq!(decoded.columns, columns);
}

/// Full loop on the real fixture: create a table, insert rows, read them
/// back through the reader boundary; every pre-existing table is unchanged.
#[test]
fn create_table_and_rows_read_back_on_the_fixture() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let before = Jet4TableReader::new(file.clone()).expect("opens");
    let before_names = before.table_names().expect("catalog");
    let before_rows: Vec<(String, usize)> = before_names
        .iter()
        .map(|n| (n.clone(), before.open_table(n).unwrap().rows.len()))
        .collect();

    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "WriterTest",
            &[
                JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 20),
                JetColumn::new("Active", JetType::Bool, 0x01, 2, 4, 0),
            ],
        )
        .expect("table created");
    writer
        .insert_row(
            tdef,
            &[
                StorageValue::Int(1),
                StorageValue::Text("first".into()),
                StorageValue::Bit(true),
            ],
        )
        .expect("row inserted");
    writer
        .insert_row(
            tdef,
            &[
                StorageValue::Int(2),
                StorageValue::Null,
                StorageValue::Bit(false),
            ],
        )
        .expect("row inserted");
    let written = writer.into_file();

    let after = Jet4TableReader::new(written).expect("written file opens");
    let names = after.table_names().expect("catalog");
    assert!(names.iter().any(|n| n == "WriterTest"), "names: {names:?}");
    let data = after.open_table("WriterTest").expect("new table reads");
    assert_eq!(data.schema.columns.len(), 3);
    assert_eq!(data.rows.len(), 2);
    assert_eq!(data.rows[0][1], StorageValue::Text("first".into()));
    assert_eq!(data.rows[0][2], StorageValue::Bit(true));
    assert_eq!(data.rows[1][1], StorageValue::Null);
    assert_eq!(data.rows[1][2], StorageValue::Bit(false));

    // Every pre-existing table is byte-for-byte unchanged in content.
    for (name, rows) in &before_rows {
        let now = after.open_table(name).expect("existing table reads");
        assert_eq!(now.rows.len(), *rows, "{name}: row count must not change");
    }
}

/// Inserting into an existing fixture table appends without disturbing the
/// old rows, and the usage maps stay consistent with the page scan.
#[test]
fn insert_into_existing_table_stays_map_consistent() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let reader = Jet4TableReader::new(file.clone()).expect("opens");
    let data = reader.open_table("Table4").expect("Table4 reads");
    let schema: Vec<String> = data.schema.columns.iter().map(|c| c.name.clone()).collect();
    let existing = data.rows.len();
    // First column of Table4 is a LongInt by schema; insert a minimal row.
    let mut values = vec![StorageValue::Null; schema.len()];
    values[0] = StorageValue::Int(99);

    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .find(|e| e.name == "Table4")
        .and_then(|e| e.tdef_page)
        .expect("Table4 tdef found");
    writer.insert_row(tdef, &values).expect("inserts");

    // Maps agree with the scan after the insert.
    for t in writer.tdef_pages() {
        let (_, map) = writer.table_used_map(t).expect("map decodes");
        let scan: Vec<u32> = writer.data_pages_of(t).collect();
        assert_eq!(map.marked_pages().collect::<Vec<_>>(), scan, "tdef {t}");
    }

    let after = Jet4TableReader::new(writer.into_file()).expect("written opens");
    let now = after.open_table("Table4").expect("reads");
    assert_eq!(now.rows.len(), existing + 1);
    for (old, new) in data.rows.iter().zip(now.rows.iter()) {
        assert_eq!(old, new, "pre-existing rows unchanged");
    }
    assert_eq!(now.rows[now.rows.len() - 1][0], StorageValue::Int(99));
}

/// A fresh table's usage page carries its (empty) used and free rows and
/// both TDEF dwords point at it.
#[test]
fn fresh_table_maps_are_wired_to_a_dedicated_usage_page() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "MapsTest",
            &[JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4)],
        )
        .expect("created");
    let (used_ref, used) = writer.table_used_map(tdef).expect("used map");
    assert_ne!(used_ref.page, 1, "dedicated usage page, not the global map");
    assert_eq!(used.marked_pages().count(), 0, "no data pages yet");
    let (free_ref, free) = writer.table_free_map(tdef).expect("free map");
    assert_eq!(free_ref.page, used_ref.page, "same usage page");
    assert_eq!(free_ref.row, used_ref.row + 1, "free row follows the used row");
    assert_eq!(free.marked_pages().count(), 0);

    // After an insert the page is owned and marked free (has room).
    writer
        .insert_row(tdef, &[StorageValue::Int(5)])
        .expect("inserts");
    let (_, used) = writer.table_used_map(tdef).expect("used map");
    assert_eq!(used.marked_pages().count(), 1);
    let (_, free) = writer.table_free_map(tdef).expect("free map");
    assert_eq!(free.marked_pages().count(), 1, "fresh page has room");
}

/// Sanity: a UsageMap decodes the way the writer's fixture probe found it
/// (kept here to pin the free-map polarity vocabulary).
#[test]
fn free_map_polarity_vocabulary() {
    let map = UsageMap::decode_row(&[0x00, 0, 0, 0, 0, 0b0000_0001, 0]).expect("decodes");
    assert!(map.owns_free(0));
    assert!(!map.owns_free(8));
}
