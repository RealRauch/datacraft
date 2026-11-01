//! Contract suite for the SQLite backend (step 13.1, item H35).
//!
//! The backend is exercised against a real, generated SQLite file — never
//! an in-memory fake (LESSONS #3). Schema, rows and foreign keys are read
//! through the same [`StoragePort`] the command core drives.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
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
    let (mut port, scratch) = open_shop();
    let opened = port.open(&scratch.0).expect("reopen the generated file");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert_eq!(
        opened.name,
        scratch.0.file_stem().unwrap().to_string_lossy()
    );
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
    assert_eq!(
        kind(&port, "customers", "name"),
        FieldKind::Text { max_len: 255 }
    );
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
    assert!(
        matches!(err, PortError::TableNotFound(ref name) if name == "missing"),
        "{err:?}"
    );
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
fn creates_a_table_and_persists_inserted_rows_across_reopen() {
    let scratch = empty_database("writes");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
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
            FieldValue::Null,
        ],
    )
    .expect("insert Grace");
    port.close();

    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0).expect("reopen the database");
    let tables = reopened.tables();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "people");
    assert_eq!(
        tables[0].fields[0].kind,
        FieldKind::AutoNumber(AutoKind::LongInteger)
    );
    let rows = reopened
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read back the inserted rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], FieldValue::Integer(1));
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][2], FieldValue::Float(1.5));
    assert_eq!(rows[1][0], FieldValue::Integer(2));
    assert_eq!(rows[1][2], FieldValue::Null);
}

#[test]
fn drop_table_removes_it_from_the_catalog_and_the_file() {
    let scratch = empty_database("drop");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
    port.create_table(&people()).expect("create people");
    assert_eq!(
        port.drop_table("PEOPLE").expect("drop case-insensitively"),
        "people"
    );
    assert!(port.tables().is_empty());
    port.close();
    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0).expect("reopen");
    assert!(reopened.tables().is_empty());
}

#[test]
fn insert_errors_are_precise() {
    let scratch = empty_database("errors");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
    port.create_table(&people()).expect("create people");

    let missing = port
        .insert_row("missing", &[])
        .expect_err("unknown table must fail");
    assert!(
        matches!(missing, PortError::TableNotFound(_)),
        "{missing:?}"
    );

    let arity = port
        .insert_row("people", &[FieldValue::Null])
        .expect_err("arity mismatch must fail");
    match arity {
        PortError::WriteFailed { detail } => {
            assert!(detail.contains("expected 3 values"), "{detail}");
            assert!(detail.contains("found 1"), "{detail}");
        }
        other => panic!("expected WriteFailed, got {other:?}"),
    }

    let drop_missing = port
        .drop_table("missing")
        .expect_err("unknown table must fail");
    assert!(
        matches!(drop_missing, PortError::TableNotFound(_)),
        "{drop_missing:?}"
    );
}

#[test]
fn create_table_rejects_an_invalid_definition() {
    let scratch = empty_database("invalid");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
    let invalid = TableDef::new("", vec![FieldDef::new("x", FieldKind::Memo)]);
    let err = port
        .create_table(&invalid)
        .expect_err("invalid definition must fail");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
}

#[test]
fn create_table_rejects_replication_id_autonumbers() {
    let scratch = empty_database("replication");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
    let def = TableDef::new(
        "guids",
        vec![FieldDef::new(
            "id",
            FieldKind::AutoNumber(AutoKind::ReplicationId),
        )],
    );
    let err = port
        .create_table(&def)
        .expect_err("SQLite has no replication-id autonumber");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn value_families_round_trip_through_sqlite_storage() {
    let scratch = empty_database("families");
    let mut port = SqlitePort::new();
    port.open(&scratch.0).expect("open empty database");
    let def = TableDef::new(
        "families",
        vec![
            FieldDef::new("flag", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("money", FieldKind::Currency),
            FieldDef::new("when", FieldKind::DateTime),
            FieldDef::new("id", FieldKind::Guid),
            FieldDef::new("blob", FieldKind::Binary(BinaryKind::OleObject)),
        ],
    );
    port.create_table(&def).expect("create families");
    let guid = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    port.insert_row(
        "families",
        &[
            FieldValue::Boolean(true),
            FieldValue::Currency(12_345),
            FieldValue::DateTime(45_000.5),
            FieldValue::Guid(guid),
            FieldValue::Binary(vec![9, 8, 7]),
        ],
    )
    .expect("insert one row of every family");
    let rows = port
        .read_rows("families", RowRange { start: 0, len: 1 })
        .expect("read the row back");
    assert_eq!(
        rows[0],
        vec![
            FieldValue::Boolean(true),
            FieldValue::Currency(12_345),
            FieldValue::DateTime(45_000.5),
            FieldValue::Guid(guid),
            FieldValue::Binary(vec![9, 8, 7]),
        ]
    );
}
