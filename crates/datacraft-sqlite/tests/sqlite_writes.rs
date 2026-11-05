//! Write contract suite for the SQLite backend (steps 13.2/13.3, H36/H37).
//!
//! Real generated files only (LESSONS #3): create/drop/insert, value-family
//! round-trips and the precise failure modes of the write path.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_sqlite::SqlitePort;

/// Unique scratch path per test process and call.
fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-w-{}-{n}-{stem}-{stamp}.sqlite",
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
fn creates_a_table_and_persists_inserted_rows_across_reopen() {
    let scratch = empty_database("writes");
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
            FieldValue::Null,
        ],
    )
    .expect("insert Grace");
    port.close();

    let mut reopened = SqlitePort::new();
    reopened
        .open(&scratch.0, None)
        .expect("reopen the database");
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
    port.open(&scratch.0, None).expect("open empty database");
    port.create_table(&people()).expect("create people");
    assert_eq!(
        port.drop_table("PEOPLE").expect("drop case-insensitively"),
        "people"
    );
    assert!(port.tables().is_empty());
    port.close();
    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    assert!(reopened.tables().is_empty());
}

#[test]
fn insert_errors_are_precise() {
    let scratch = empty_database("errors");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
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
    port.open(&scratch.0, None).expect("open empty database");
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
    port.open(&scratch.0, None).expect("open empty database");
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
    port.open(&scratch.0, None).expect("open empty database");
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

#[test]
fn every_domain_kind_creates_and_reads_back() {
    let scratch = empty_database("kinds");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    let def = TableDef::new(
        "kinds",
        vec![
            FieldDef::new("txt", FieldKind::Text { max_len: 10 }),
            FieldDef::new("memo", FieldKind::Memo),
            FieldDef::new("byte", FieldKind::Number(NumericKind::Byte)),
            FieldDef::new("int", FieldKind::Number(NumericKind::Integer)),
            FieldDef::new("long", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("single", FieldKind::Number(NumericKind::Single)),
            FieldDef::new("double", FieldKind::Number(NumericKind::Double)),
            FieldDef::new(
                "dec",
                FieldKind::Number(NumericKind::Decimal {
                    precision: 10,
                    scale: 2,
                }),
            ),
            FieldDef::new("when", FieldKind::DateTime),
            FieldDef::new("money", FieldKind::Currency),
            FieldDef::new("flag", FieldKind::Boolean { tri_state: true }),
            FieldDef::new("blob", FieldKind::Binary(BinaryKind::OleObject)),
            FieldDef::new("guid", FieldKind::Guid),
        ],
    );
    port.create_table(&def).expect("create every kind");
    let kinds: Vec<FieldKind> = port.tables()[0].fields.iter().map(|f| f.kind).collect();
    assert_eq!(kinds[1], FieldKind::Memo);
    assert_eq!(kinds[4], FieldKind::Number(NumericKind::LongInteger));
    assert_eq!(kinds[5], FieldKind::Number(NumericKind::Single));
    assert_eq!(
        kinds[7],
        FieldKind::Number(NumericKind::Decimal {
            precision: 10,
            scale: 2
        })
    );

    port.insert_row(
        "kinds",
        &[
            FieldValue::Text("t".into()),
            FieldValue::Text("long".into()),
            FieldValue::Integer(1),
            FieldValue::Integer(2),
            FieldValue::Integer(3),
            FieldValue::Float(1.25),
            FieldValue::Float(2.5),
            FieldValue::Decimal(1234),
            FieldValue::DateTime(45_000.25),
            FieldValue::Currency(99),
            FieldValue::Boolean(false),
            FieldValue::Binary(vec![1]),
            FieldValue::Guid([3; 16]),
        ],
    )
    .expect("insert every kind");
    let rows = port
        .read_rows("kinds", RowRange { start: 0, len: 1 })
        .expect("read every kind");
    assert_eq!(rows[0][2], FieldValue::Integer(1));
    assert_eq!(rows[0][5], FieldValue::Float(1.25));
    assert_eq!(rows[0][6], FieldValue::Float(2.5));
    assert_eq!(rows[0][8], FieldValue::DateTime(45_000.25));
    assert_eq!(rows[0][9], FieldValue::Currency(99));
    assert_eq!(rows[0][10], FieldValue::Boolean(false));
    assert_eq!(rows[0][11], FieldValue::Binary(vec![1]));
    assert_eq!(rows[0][12], FieldValue::Guid([3; 16]));
}

#[test]
fn a_table_with_only_an_autonumber_inserts_default_values() {
    let scratch = empty_database("defaults");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    let def = TableDef::new(
        "counters",
        vec![FieldDef::new(
            "id",
            FieldKind::AutoNumber(AutoKind::LongInteger),
        )],
    );
    port.create_table(&def).expect("create counters");
    port.insert_row("counters", &[FieldValue::Null])
        .expect("default values insert");
    port.insert_row("counters", &[FieldValue::Null])
        .expect("second default values insert");
    let rows = port
        .read_rows("counters", RowRange { start: 0, len: 10 })
        .expect("read counters");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][0], FieldValue::Integer(2));
}

#[test]
fn writes_fail_precisely_for_duplicates_not_null_and_user_autonumbers() {
    let scratch = empty_database("failures");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");
    let mut def = TableDef::new(
        "strict",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 10 }),
        ],
    );
    def.fields[1].properties.required = true;
    port.create_table(&def).expect("create strict");
    let duplicate = port
        .create_table(&def)
        .expect_err("duplicate table must fail");
    assert!(
        matches!(duplicate, PortError::WriteFailed { .. }),
        "{duplicate:?}"
    );

    let not_null = port
        .insert_row("strict", &[FieldValue::Null, FieldValue::Null])
        .expect_err("NOT NULL rejects Null");
    assert!(
        matches!(not_null, PortError::WriteFailed { .. }),
        "{not_null:?}"
    );

    let user_id = port
        .insert_row(
            "strict",
            &[FieldValue::Integer(5), FieldValue::Text("Ada".into())],
        )
        .expect_err("user autonumber values are rejected");
    match user_id {
        PortError::WriteFailed { detail } => {
            assert!(detail.contains("engine-assigned"), "{detail}");
        }
        other => panic!("expected WriteFailed, got {other:?}"),
    }
}

#[test]
fn foreign_keys_resolve_the_referenced_primary_key() {
    let path = scratch_path("fk");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE parents (id INTEGER PRIMARY KEY, label TEXT);
         CREATE TABLE children (id INTEGER PRIMARY KEY,
             parent INTEGER REFERENCES parents);",
    )
    .expect("populate foreign keys");
    let _scratch = Scratch(path.clone());

    let mut port = SqlitePort::new();
    port.open(&path, None).expect("open the database");
    let relationships = port.relationships().expect("read foreign keys");
    assert_eq!(
        relationships,
        vec![datacraft_core::port::Relationship {
            table: "children".into(),
            column: "parent".into(),
            related_table: "parents".into(),
            related_column: "id".into(),
        }]
    );
}

#[test]
fn an_unresolvable_foreign_key_is_reported() {
    let path = scratch_path("fk-unresolved");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE orphans (x INTEGER);
         CREATE TABLE loose (y INTEGER REFERENCES orphans);",
    )
    .expect("populate the unresolved key");
    let _scratch = Scratch(path.clone());

    let mut port = SqlitePort::new();
    port.open(&path, None).expect("open the database");
    let err = port
        .relationships()
        .expect_err("the unresolved key must be reported");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn opening_a_missing_file_and_the_path_accessor_are_exact() {
    let path = scratch_path("missing");
    let _ = std::fs::remove_file(&path);
    let mut port = SqlitePort::new();
    let err = port.open(&path, None).expect_err("missing files fail");
    assert!(matches!(err, PortError::OpenFailed { .. }), "{err:?}");
    assert!(port.path().is_none());

    let scratch = empty_database("accessor");
    port.open(&scratch.0, None).expect("open existing file");
    assert_eq!(port.path(), Some(scratch.0.as_path()));
    port.close();
    assert!(port.path().is_none());
}

#[test]
fn guid_columns_read_text_and_non_guid_blobs() {
    let path = scratch_path("guidtext");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE things (id GUID, payload GUID);
         INSERT INTO things VALUES ('01234567-89ab-cdef-fedc-ba9876543210', x'0102');
         INSERT INTO things VALUES ('plain', x'0123456789abcdef0123456789abcdef');",
    )
    .expect("populate guid values");
    let _scratch = Scratch(path.clone());

    let mut port = SqlitePort::new();
    port.open(&path, None).expect("open the database");
    let rows = port
        .read_rows("things", RowRange { start: 0, len: 10 })
        .expect("read guid values");
    assert_eq!(
        rows[0][0],
        FieldValue::Guid([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ])
    );
    assert_eq!(rows[0][1], FieldValue::Binary(vec![1, 2]));
    assert_eq!(rows[1][0], FieldValue::Text("plain".into()));
    assert_eq!(
        rows[1][1],
        FieldValue::Guid([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ])
    );
}
