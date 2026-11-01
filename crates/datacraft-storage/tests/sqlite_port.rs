//! Contract suite for the SQLite backend (step 13.1, item H35).
//!
//! The backend is exercised against a real, generated SQLite file — never
//! an in-memory fake (LESSONS #3). Schema, rows and foreign keys are read
//! through the same [`StoragePort`] the command core drives.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{AutoKind, BinaryKind, FieldKind, FieldValue, NumericKind};
use datacraft_storage::sqlite::SqlitePort;

/// Unique scratch path per test process and call.
fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-{}-{n}-{stem}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Creates the shop database the suite reads.
fn shop_database() -> Scratch {
    let path = scratch_path("shop");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE customers (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT NOT NULL,
             code VARCHAR(12),
             balance REAL,
             active BOOLEAN,
             note BLOB,
             amount NUMERIC
         );
         CREATE TABLE orders (
             id INTEGER PRIMARY KEY,
             customer_id INTEGER NOT NULL REFERENCES customers(id),
             total REAL
         );
         INSERT INTO customers (name, code, balance, active, note, amount)
             VALUES ('Ada', 'A-1', 1.5, 1, x'010203', 42);
         INSERT INTO customers (name, code, balance, active, note, amount)
             VALUES ('Grace', NULL, NULL, 0, NULL, NULL);
         INSERT INTO orders (id, customer_id, total) VALUES (10, 1, 19.5);
         INSERT INTO orders (id, customer_id, total) VALUES (11, 1, 4.25);",
    )
    .expect("populate scratch database");
    Scratch(path)
}

fn open_shop() -> (SqlitePort, Scratch) {
    let scratch = shop_database();
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open generated sqlite file");
    (port, scratch)
}

fn field(port: &SqlitePort, table: &str, name: &str) -> datacraft_core::schema::FieldDef {
    port.tables()
        .into_iter()
        .find(|t| t.name == table)
        .and_then(|t| t.fields.into_iter().find(|f| f.name == name))
        .expect("field exists")
}

fn kind(port: &SqlitePort, table: &str, name: &str) -> FieldKind {
    field(port, table, name).kind
}

#[test]
fn opens_a_sqlite_file_with_the_sqlite_format() {
    let (port, scratch) = open_shop();
    let opened = port
        .open(&scratch.0)
        .expect("reopen the generated file");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert_eq!(opened.name, scratch.0.file_stem().unwrap().to_string_lossy());
}

#[test]
fn lists_user_tables_in_creation_order_without_internal_tables() {
    let (port, _scratch) = open_shop();
    let names: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
    assert_eq!(names, ["customers", "orders"]);
}

#[test]
fn maps_declared_types_onto_access_kinds() {
    let (port, _scratch) = open_shop();
    assert_eq!(
        kind(&port, "customers", "id"),
        FieldKind::AutoNumber(AutoKind::LongInteger)
    );
    assert_eq!(kind(&port, "customers", "name"), FieldKind::Text { max_len: 255 });
    assert_eq!(
        kind(&port, "customers", "code"),
        FieldKind::Text { max_len: 12 }
    );
    assert_eq!(
        kind(&port, "customers", "balance"),
        FieldKind::Number(NumericKind::Double)
    );
    assert_eq!(
        kind(&port, "customers", "active"),
        FieldKind::Boolean { tri_state: true }
    );
    assert_eq!(
        kind(&port, "customers", "note"),
        FieldKind::Binary(BinaryKind::OleObject)
    );
    assert_eq!(
        kind(&port, "customers", "amount"),
        FieldKind::Number(NumericKind::Decimal {
            precision: 28,
            scale: 0
        })
    );
    assert_eq!(
        kind(&port, "orders", "id"),
        FieldKind::Number(NumericKind::LongInteger)
    );
}

#[test]
fn not_null_columns_are_required_fields() {
    let (port, _scratch) = open_shop();
    assert!(field(&port, "customers", "name").properties.required);
    assert!(field(&port, "customers", "id").properties.required);
    assert!(!field(&port, "customers", "code").properties.required);
}

#[test]
fn reads_rows_with_null_and_value_family_fidelity() {
    let (port, _scratch) = open_shop();
    let rows = port
        .read_rows("customers", RowRange { start: 0, len: 10 })
        .expect("read customers");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Text("A-1".into()),
            FieldValue::Float(1.5),
            FieldValue::Boolean(true),
            FieldValue::Binary(vec![1, 2, 3]),
            FieldValue::Decimal(42),
        ]
    );
    assert_eq!(
        rows[1],
        vec![
            FieldValue::Integer(2),
            FieldValue::Text("Grace".into()),
            FieldValue::Null,
            FieldValue::Null,
            FieldValue::Boolean(false),
            FieldValue::Null,
            FieldValue::Null,
        ]
    );
}

#[test]
fn windows_start_at_the_requested_row_and_clamp_at_the_end() {
    let (port, _scratch) = open_shop();
    let rows = port
        .read_rows("orders", RowRange { start: 1, len: 10 })
        .expect("tail window");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], FieldValue::Integer(11));

    let empty = port
        .read_rows("orders", RowRange { start: 2, len: 5 })
        .expect("window starting at the row count is empty");
    assert!(empty.is_empty());

    let err = port
        .read_rows("orders", RowRange { start: 3, len: 5 })
        .expect_err("window beyond the row count is out of range");
    assert!(matches!(err, PortError::OutOfRange { .. }), "{err:?}");
}

#[test]
fn relationships_come_from_foreign_keys() {
    let (port, _scratch) = open_shop();
    let relationships = port.relationships().expect("read foreign keys");
    assert_eq!(
        relationships,
        vec![datacraft_core::port::Relationship {
            table: "orders".into(),
            column: "customer_id".into(),
            related_table: "customers".into(),
            related_column: "id".into(),
        }]
    );
}

#[test]
fn unknown_tables_are_reported_precisely() {
    let (port, _scratch) = open_shop();
    let err = port
        .read_rows("missing", RowRange { start: 0, len: 1 })
        .expect_err("unknown table must fail");
    assert!(matches!(err, PortError::TableNotFound(name) if name == "missing"), "{err:?}");
}

#[test]
fn opening_a_file_that_is_not_sqlite_fails_with_a_precise_error() {
    let path = scratch_path("junk");
    std::fs::write(&path, b"not a database at all").expect("write junk");
    let _scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    let err = port.open(&path).expect_err("junk must be rejected");
    match err {
        PortError::OpenFailed { detail } => {
            assert!(detail.contains("SQLite"), "{detail}");
            assert!(detail.contains(&path.display().to_string()), "{detail}");
        }
        other => panic!("expected OpenFailed, got {other:?}"),
    }
}

#[test]
fn reading_without_an_open_database_is_unsupported() {
    let port = SqlitePort::new();
    let err = port
        .read_rows("customers", RowRange { start: 0, len: 1 })
        .expect_err("no database open");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn closing_releases_the_connection() {
    let (mut port, _scratch) = open_shop();
    port.close();
    assert!(port.tables().is_empty());
    let err = port
        .read_rows("customers", RowRange { start: 0, len: 1 })
        .expect_err("closed port has no database");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}
