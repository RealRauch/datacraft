//! Jet4 row update/delete contract (step 11.10, item H40).
//!
//! Every test writes a private copy of the pinned fixture (fixtures are
//! immutable evidence, docs/TESTING.md §4). Updates and deletes prove
//! themselves through a reopen; where `mdb-export` is installed the oracle
//! additionally cross-checks the written file.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_storage::StorageError;
use datacraft_storage::StorageValue;
use datacraft_storage::jet::{DataPage, Jet4TableReader, Jet4Writer, JetColumn, JetType};
use datacraft_storage::port_adapter::Jet4Port;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// A private copy of the pinned fixture (never write the original).
fn scratch_copy(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "datacraft-jet-rows-{}-{n}-{stem}.mdb",
        std::process::id()
    ));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn opened(path: &Path) -> Jet4Port {
    let mut port = Jet4Port::new();
    port.open(path).expect("open the fixture copy");
    port
}

fn table1_values() -> Vec<FieldValue> {
    vec![
        FieldValue::Text("updated".into()),
        FieldValue::Text("second".into()),
        FieldValue::Integer(9),
        FieldValue::Integer(8),
        FieldValue::Integer(7),
        FieldValue::Float(6.5),
        FieldValue::DateTime(45_000.0),
        FieldValue::Currency(12_500),
        FieldValue::Boolean(true),
    ]
}

fn read_all(port: &Jet4Port, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: 1000,
        },
    )
    .expect("read rows")
}

#[test]
fn update_row_persists_across_a_reopen() {
    let scratch = Scratch(scratch_copy("update"));
    let mut port = opened(&scratch.0);
    let before = read_all(&port, "Table1");
    assert!(before.len() >= 2, "fixture carries at least two rows");
    let untouched = before[1].clone();

    port.update_row("Table1", 0, &table1_values())
        .expect("update row 0");
    let after = read_all(&port, "Table1");
    assert_eq!(after.len(), before.len(), "update keeps the row count");
    assert_eq!(after[0], table1_values());
    assert_eq!(after[1], untouched, "other rows are untouched");
    port.close();

    let reopened = opened(&scratch.0);
    let rows = read_all(&reopened, "Table1");
    assert_eq!(rows[0], table1_values());
    assert_eq!(rows[1], untouched);
}

#[test]
fn updating_a_fixed_width_column_keeps_the_row_slot() {
    let scratch = Scratch(scratch_copy("inplace"));
    let mut port = opened(&scratch.0);
    let mut values = read_all(&port, "Table1")[0].clone();
    // Column E is a fixed-width Long Integer: the encoded row length is
    // unchanged, so the writer takes the in-place path.
    values[4] = FieldValue::Integer(424_242);
    port.update_row("Table1", 0, &values).expect("update E");
    port.close();

    let reopened = opened(&scratch.0);
    assert_eq!(
        read_all(&reopened, "Table1")[0][4],
        FieldValue::Integer(424_242)
    );
}

#[test]
fn delete_row_removes_exactly_one_row() {
    let scratch = Scratch(scratch_copy("delete"));
    let mut port = opened(&scratch.0);
    let before = read_all(&port, "Table1");
    assert!(before.len() >= 2, "fixture carries at least two rows");

    port.delete_row("Table1", 1).expect("delete row 1");
    let after = read_all(&port, "Table1");
    assert_eq!(after.len(), before.len() - 1);
    assert_eq!(after[0], before[0]);
    port.close();

    let reopened = opened(&scratch.0);
    let rows = read_all(&reopened, "Table1");
    assert_eq!(rows.len(), before.len() - 1);
    assert_eq!(rows[0], before[0]);
}

#[test]
fn deleting_every_row_leaves_an_empty_table() {
    let scratch = Scratch(scratch_copy("delete-all"));
    let mut port = opened(&scratch.0);
    let before = read_all(&port, "Table1");
    for _ in 0..before.len() {
        port.delete_row("Table1", 0).expect("delete the first row");
    }
    assert!(read_all(&port, "Table1").is_empty());
    port.close();

    let reopened = opened(&scratch.0);
    assert!(read_all(&reopened, "Table1").is_empty());
}

#[test]
fn row_indices_beyond_the_table_are_rejected() {
    let scratch = Scratch(scratch_copy("bounds"));
    let mut port = opened(&scratch.0);
    let count = read_all(&port, "Table1").len();

    let err = port
        .update_row("Table1", count + 5, &table1_values())
        .expect_err("update out of range");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
    let err = port
        .delete_row("Table1", count + 5)
        .expect_err("delete out of range");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
    assert_eq!(read_all(&port, "Table1").len(), count);
}

#[test]
fn memo_and_ole_rows_update_and_delete() {
    let scratch = Scratch(scratch_copy("memo"));
    let mut port = opened(&scratch.0);
    let notes = TableDef::new(
        "Notes",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("note", FieldKind::Memo),
            FieldDef::new("blob", FieldKind::Binary(BinaryKind::OleObject)),
        ],
    );
    port.create_table(&notes).expect("create Notes");
    port.insert_row(
        "Notes",
        &[
            FieldValue::Integer(1),
            FieldValue::Text("short".into()),
            FieldValue::Binary(vec![1, 2, 3]),
        ],
    )
    .expect("insert row 1");
    port.insert_row(
        "Notes",
        &[
            FieldValue::Integer(2),
            FieldValue::Text("other".into()),
            FieldValue::Null,
        ],
    )
    .expect("insert row 2");

    port.update_row(
        "Notes",
        0,
        &[
            FieldValue::Integer(1),
            FieldValue::Text("a much longer memo body that forces a page repack".into()),
            FieldValue::Binary(vec![9, 8, 7, 6, 5]),
        ],
    )
    .expect("update the memo/ole row");
    port.delete_row("Notes", 1).expect("delete the second row");
    port.close();

    let reopened = opened(&scratch.0);
    let rows = read_all(&reopened, "Notes");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], FieldValue::Integer(1));
    assert_eq!(
        rows[0][1],
        FieldValue::Text("a much longer memo body that forces a page repack".into())
    );
    assert_eq!(rows[0][2], FieldValue::Binary(vec![9, 8, 7, 6, 5]));
}

#[test]
fn updates_and_deletes_keep_the_usage_maps_consistent() {
    let scratch = Scratch(scratch_copy("maps"));
    let mut port = opened(&scratch.0);
    port.update_row("Table1", 0, &table1_values())
        .expect("update row 0");
    port.delete_row("Table1", 1).expect("delete row 1");
    port.close();

    let bytes = std::fs::read(&scratch.0).expect("read the written file");
    let writer = Jet4Writer::new(bytes).expect("the written file opens");
    writer.global_free_map().expect("global free map decodes");
    let heads = writer.tdef_pages();
    assert!(!heads.is_empty(), "tables survive");
    for head in heads {
        writer.table_used_map(head).expect("used map decodes");
        writer.table_free_map(head).expect("free map decodes");
    }
}

#[test]
fn autonumber_values_are_written_by_an_update() {
    let scratch = Scratch(scratch_copy("autonumber"));
    // Seed Table3 (a: AutoNumber, b: Short Text) through the raw writer:
    // the insert path rejects AutoNumbers, the update path preserves them.
    let bytes = std::fs::read(&scratch.0).expect("read fixture");
    let reader = Jet4TableReader::new(bytes.clone()).expect("reader");
    let page = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .find(|entry| entry.name == "Table3")
        .and_then(|entry| entry.tdef_page)
        .expect("Table3 is catalogued");
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("seed".into())],
        )
        .expect("seed the autonumber row");
    std::fs::write(&scratch.0, writer.into_file()).expect("persist the seed");

    let mut port = opened(&scratch.0);
    port.update_row(
        "Table3",
        0,
        &[FieldValue::Integer(1), FieldValue::Text("renamed".into())],
    )
    .expect("update keeps the autonumber value");
    port.close();

    let reopened = opened(&scratch.0);
    let rows = read_all(&reopened, "Table3");
    assert_eq!(rows[0][0], FieldValue::Integer(1));
    assert_eq!(rows[0][1], FieldValue::Text("renamed".into()));
}

/// The oracle check is optional: it runs where `mdb-export` is installed.
#[test]
fn the_oracle_reads_a_written_file() {
    let scratch = Scratch(scratch_copy("oracle"));
    let mut port = opened(&scratch.0);
    port.update_row("Table1", 0, &table1_values())
        .expect("update row 0");
    port.close();

    let Ok(output) = Command::new("mdb-export")
        .arg(&scratch.0)
        .arg("Table1")
        .output()
    else {
        eprintln!("oracle skipped: mdb-export not installed");
        return;
    };
    assert!(
        output.status.success(),
        "mdb-export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let csv = String::from_utf8_lossy(&output.stdout);
    assert!(csv.contains("updated"), "{csv}");
}

#[test]
fn update_errors_are_precise() {
    let scratch = Scratch(scratch_copy("update-errors"));
    let mut port = opened(&scratch.0);

    let err = port
        .update_row("Table1", 0, &[FieldValue::Integer(1)])
        .expect_err("arity");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
    assert!(err.to_string().contains("defines 9 fields"), "{err}");

    let mut values = table1_values();
    values[2] = FieldValue::Text("not a byte".into());
    let err = port.update_row("Table1", 0, &values).expect_err("kind");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");

    // Table3's AutoNumber column accepts only its engine-assigned integer.
    let err = port
        .update_row(
            "Table3",
            0,
            &[FieldValue::Text("id".into()), FieldValue::Text("b".into())],
        )
        .expect_err("autonumber type");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
    assert!(err.to_string().contains("AutoNumber"), "{err}");
}

#[test]
fn row_operations_require_an_open_database() {
    let mut port = Jet4Port::new();
    assert!(matches!(
        port.update_row("Table1", 0, &[FieldValue::Null]),
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        port.delete_row("Table1", 0),
        Err(PortError::Unsupported { .. })
    ));
}

#[test]
fn writer_row_updates_cover_in_place_external_and_full_pages() {
    let scratch = Scratch(scratch_copy("writer-rows"));
    let bytes = std::fs::read(&scratch.0).expect("read fixture");
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    let page = writer
        .create_table(
            "RowOps",
            &[
                JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("note", JetType::Memo, 0x02, 1, 0, 0),
            ],
        )
        .expect("create the table");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("abcde".into())],
        )
        .expect("insert the row");

    // Same-length text: the writer overwrites the slot in place.
    writer
        .update_row(
            page,
            0,
            &[StorageValue::Int(1), StorageValue::Text("vwxyz".into())],
        )
        .expect("in-place update");

    // A row that no longer fits the page answers a precise Unsupported.
    let huge = "x".repeat(5000);
    let err = writer
        .update_row(page, 0, &[StorageValue::Int(1), StorageValue::Text(huge)])
        .expect_err("row larger than the page");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("does not fit"), "{err}");

    // A row that fits the page but collides with the offset table too.
    let tight = "x".repeat(2035);
    let err = writer
        .update_row(page, 0, &[StorageValue::Int(1), StorageValue::Text(tight)])
        .expect_err("row collides with the offset table");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");

    // Patch the memo header into the external LVAL form: the update must be
    // refused instead of orphaning the chain.
    let mut bytes = writer.into_file();
    let (page_no, slot) = Jet4Writer::new(bytes.clone())
        .expect("writer")
        .row_location(page, 0)
        .expect("locate the row");
    let base = page_no as usize * 4096;
    let decoded = DataPage::decode(&bytes[base..base + 4096]).expect("decode the page");
    let (start, _, _) = decoded.records[slot];
    let memo_at = base + start + 2 + 4; // num_cols + fixed area
    bytes[memo_at..memo_at + 4].copy_from_slice(&0x4000_0000u32.to_le_bytes());
    let mut patched = Jet4Writer::new(bytes).expect("writer");
    let err = patched
        .update_row(
            page,
            0,
            &[StorageValue::Int(1), StorageValue::Text("new".into())],
        )
        .expect_err("external LVAL row");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("LVAL"), "{err}");
}

#[test]
fn rows_behind_an_overflow_pointer_are_not_updated() {
    let scratch = Scratch(scratch_copy("overflow"));
    let bytes = std::fs::read(&scratch.0).expect("read fixture");
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    let page = writer
        .create_table(
            "Overflow",
            &[
                JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("note", JetType::Memo, 0x02, 1, 0, 0),
            ],
        )
        .expect("create the table");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("first".into())],
        )
        .expect("insert the data row");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(2), StorageValue::Text("second".into())],
        )
        .expect("insert the pointer row");

    // Rewire slot 1 into an overflow source that points at slot 0 (the
    // target), exactly as Access stores a row larger than one page.
    let mut bytes = writer.into_file();
    let (page_no, _) = Jet4Writer::new(bytes.clone())
        .expect("writer")
        .row_location(page, 0)
        .expect("locate the first row");
    let base = page_no as usize * 4096;
    let decoded = DataPage::decode(&bytes[base..base + 4096]).expect("decode the page");
    let (_, _, raw0) = decoded.records[0];
    let (start1, _, raw1) = decoded.records[1];
    let pointer = (page_no << 8).to_le_bytes();
    bytes[base + start1..base + start1 + 4].copy_from_slice(&pointer);
    bytes[base + 14 + 2..base + 16 + 2].copy_from_slice(&(raw1 | 0x4000).to_le_bytes());
    bytes[base + 14..base + 16].copy_from_slice(&(raw0 | 0x8000).to_le_bytes());

    let mut patched = Jet4Writer::new(bytes).expect("writer");
    let err = patched
        .update_row(
            page,
            0,
            &[StorageValue::Int(9), StorageValue::Text("new".into())],
        )
        .expect_err("overflow row");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("overflow"), "{err}");
}
