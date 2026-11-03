//! SQLite row update/delete contracts (step 11.10, item H40).
//!
//! Real generated files only (LESSONS #3): row operations address the
//! live-row order and persist across a reopen; AutoNumber columns keep
//! their engine-assigned value.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_sqlite::SqlitePort;

/// Unique scratch path per test process and call.
fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-rowops-{}-{n}-{stem}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Creates an empty but initialized SQLite file (header written).
fn empty_database(stem: &str) -> Scratch {
    let path = scratch_path(stem);
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create empty scratch database");
    conn.execute_batch("CREATE TABLE seed (x INTEGER); DROP TABLE seed;")
        .expect("force the SQLite header");
    Scratch(path)
}

fn people() -> TableDef {
    TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

#[test]
fn updates_and_deletes_persist_across_reopen() {
    let scratch = empty_database("rowops");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    port.create_table(&people()).expect("create people");
    port.insert_row(
        "people",
        &[
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )
    .expect("insert Ada");
    port.insert_row(
        "people",
        &[
            FieldValue::Null,
            FieldValue::Text("Grace".into()),
            FieldValue::Float(2.5),
        ],
    )
    .expect("insert Grace");

    // AutoNumber values are preserved by the update (Null keeps the id).
    port.update_row(
        "people",
        0,
        &[
            FieldValue::Null,
            FieldValue::Text("Ada L.".into()),
            FieldValue::Float(9.5),
        ],
    )
    .expect("update Ada");
    port.delete_row("people", 1).expect("delete Grace");
    port.close();

    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    let rows = reopened
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], FieldValue::Integer(1), "autonumber preserved");
    assert_eq!(rows[0][1], FieldValue::Text("Ada L.".into()));
    assert_eq!(rows[0][2], FieldValue::Float(9.5));
}

#[test]
fn row_operations_out_of_range_are_rejected() {
    let scratch = empty_database("rowops-bounds");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    port.create_table(&people()).expect("create people");
    port.insert_row(
        "people",
        &[
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )
    .expect("insert Ada");

    let err = port
        .update_row(
            "people",
            5,
            &[
                FieldValue::Null,
                FieldValue::Text("Ghost".into()),
                FieldValue::Float(0.0),
            ],
        )
        .expect_err("update out of range");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
    let err = port
        .delete_row("people", 5)
        .expect_err("delete out of range");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
    assert_eq!(
        port.read_rows("people", RowRange { start: 0, len: 10 })
            .expect("rows stay")
            .len(),
        1
    );
}

#[test]
fn update_arity_and_autonumber_only_tables() {
    let scratch = empty_database("rowops-arity");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    port.create_table(&people()).expect("create people");
    port.insert_row(
        "people",
        &[
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )
    .expect("insert Ada");

    let err = port
        .update_row(
            "people",
            0,
            &[FieldValue::Null, FieldValue::Text("Ada".into())],
        )
        .expect_err("arity");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");

    // A table with only an AutoNumber column has nothing to assign; the
    // update just verifies the row exists.
    let only_id = TableDef::new(
        "only_id",
        vec![FieldDef::new(
            "id",
            FieldKind::AutoNumber(AutoKind::LongInteger),
        )],
    );
    port.create_table(&only_id).expect("create only_id");
    port.insert_row("only_id", &[FieldValue::Null])
        .expect("insert default row");
    port.update_row("only_id", 0, &[FieldValue::Null])
        .expect("update the autonumber-only row");
    let err = port
        .update_row("only_id", 4, &[FieldValue::Null])
        .expect_err("bounds");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
}

#[test]
fn restore_row_reproduces_the_autonumber_identity() {
    let scratch = empty_database("restore");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    port.create_table(&people()).expect("create");
    port.insert_row(
        "people",
        &[
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )
    .expect("insert");
    let before = port
        .read_rows(
            "people",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read")[0]
        .clone();
    assert!(matches!(before[0], FieldValue::Integer(_)));
    port.delete_row("people", 0).expect("delete");
    port.restore_row("people", &before).expect("restore");
    let rows = port
        .read_rows(
            "people",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read restored");
    assert_eq!(rows, vec![before]);
    port.close();

    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    assert_eq!(
        reopened
            .read_rows(
                "people",
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .expect("read after reopen"),
        rows
    );

    let error = reopened
        .restore_row(
            "people",
            &[
                FieldValue::Null,
                FieldValue::Text("engine".into()),
                FieldValue::Float(0.0),
            ],
        )
        .expect_err("a restore needs the identity");
    assert!(error.to_string().contains("AutoNumber"), "{error}");
}
