//! Driver capability invariants over the SQLite backend (phase 30, item
//! M17; ADR-7): the driver names itself, reports honest per-session
//! capabilities (a read-only file disables the write capabilities), and
//! every reported `No` answers on the matching port call. RED state:
//! `SqlitePort` does not override the capability reporting yet — it
//! answers the optimistic trait defaults.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DriverId, PortError, RowRange, SchemaChange, StoragePort, Support};
use datacraft_core::schema::FieldValue;
use datacraft_storage::sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-caps-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn writable_database() -> Scratch {
    let path = scratch_path("data.sqlite");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);
         INSERT INTO notes (body) VALUES ('hello');",
    )
    .expect("populate scratch database");
    Scratch(path)
}

/// The SQLite driver reports its identity and its honest writable set:
/// no schema alteration, no atomic batches, no passwords, no Jet-style
/// inventories — but engine-assigned AutoNumbers and relationship reads.
#[test]
fn the_sqlite_driver_reports_its_writable_capabilities() {
    let scratch = writable_database();
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open the database");
    assert_eq!(port.driver_id(), DriverId::new("sqlite"));
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.write_rows);
    assert!(caps.create_table);
    assert!(caps.drop_table);
    assert!(caps.create_database);
    assert!(!caps.alter_table, "SQLite has no alter path yet");
    assert!(!caps.atomic_batch, "batches apply one by one");
    assert!(!caps.passwords, "SQLite has no database password");
    assert_eq!(caps.autonumber, Support::Yes);
    assert_eq!(caps.relationships, Support::ReadOnly);
    assert_eq!(caps.native_queries, Support::No);
    assert_eq!(caps.macros, Support::No);
    assert_eq!(caps.vba, Support::No);
    assert_eq!(caps.triggers, Support::No);
    assert_eq!(caps.stored_procedures, Support::No);
}

/// A read-only file opens fine and disables exactly the write
/// capabilities.
#[test]
fn a_read_only_sqlite_file_reports_read_only_capabilities() {
    let scratch = writable_database();
    let mut perms = std::fs::metadata(&scratch.0)
        .expect("metadata")
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&scratch.0, perms).expect("mark read-only");

    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open read-only");
    let caps = port.capabilities();
    assert!(caps.read_tables);
    assert!(caps.create_database, "creation is session-independent");
    assert!(!caps.write_rows);
    assert!(!caps.create_table);
    assert!(!caps.drop_table);

    // The declared read-only state is a contract on the calls, too.
    let error = port
        .insert_row("notes", &[FieldValue::Null, FieldValue::Text("x".to_owned())])
        .expect_err("insert is refused on a read-only file");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    let error = port
        .read_rows("notes", RowRange { start: 0, len: 10 })
        .expect("reads still work");
    assert_eq!(error.len(), 1);

    let mut perms = std::fs::metadata(&scratch.0)
        .expect("metadata")
        .permissions();
    perms.set_readonly(false);
    std::fs::set_permissions(&scratch.0, perms).expect("restore writable for cleanup");
}

/// Reported gaps answer precisely on the matching calls: no alter path,
/// no native queries, no macros, no VBA.
#[test]
fn sqlite_capability_gaps_answer_precisely() {
    let scratch = writable_database();
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open the database");

    let error = port
        .alter_table(
            "notes",
            SchemaChange::RenameField {
                field: "body".to_owned(),
                new_name: "text".to_owned(),
            },
        )
        .expect_err("alter_table is unsupported on SQLite");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    assert!(
        port.native_queries().expect("default").is_empty(),
        "no native-query inventory"
    );
    assert!(port.macros().expect("default").is_empty(), "no macros");
    assert!(port.vba_project().expect("default").is_none(), "no VBA");
}

/// A reported `Yes` is honoured: file creation works through the
/// reported capability.
#[test]
fn sqlite_create_database_capability_is_honoured() {
    let mut port = SqlitePort::new();
    assert!(port.capabilities().create_database);
    let path = scratch_path("created.sqlite");
    let _ = std::fs::remove_file(&path);
    port.create_database(&path, datacraft_core::port::DatabaseFormat::Sqlite)
        .expect("create the SQLite file");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}
