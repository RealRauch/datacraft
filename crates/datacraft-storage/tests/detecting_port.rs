//! Detection contract of the composed backend (step 13.1, item H35).
//!
//! One port decides by file magic which backend serves the file, so every
//! client (GUI, MCP, CLI, E2E) can open Jet/ACE and SQLite files through
//! the same command vocabulary.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_storage::detecting::DetectingPort;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-detect-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn sqlite_database() -> Scratch {
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

#[test]
fn detects_a_jet4_fixture() {
    let mut port = DetectingPort::new();
    let opened = port.open(&fixture(), None).expect("open the Jet4 fixture");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert_eq!(opened.name, "testV2000");
    assert!(!port.tables().is_empty());
}

#[test]
fn detects_a_sqlite_file_and_serves_its_rows() {
    let scratch = sqlite_database();
    let mut port = DetectingPort::new();
    let opened = port.open(&scratch.0, None).expect("open the SQLite file");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert_eq!(port.tables().len(), 1);
    let rows = port
        .read_rows("notes", RowRange { start: 0, len: 10 })
        .expect("read through the composed port");
    assert_eq!(rows[0][1], FieldValue::Text("hello".into()));
}

#[test]
fn a_jet_file_and_a_sqlite_file_can_follow_each_other() {
    let scratch = sqlite_database();
    let mut port = DetectingPort::new();
    let jet = port.open(&fixture(), None).expect("open Jet4");
    assert_eq!(jet.format, DatabaseFormat::Jet4);
    let sqlite = port.open(&scratch.0, None).expect("open SQLite");
    assert_eq!(sqlite.format, DatabaseFormat::Sqlite);
    assert_eq!(port.tables().len(), 1);
}

#[test]
fn rejects_unknown_files_with_a_precise_error() {
    let path = scratch_path("junk.bin");
    std::fs::write(&path, b"neither jet nor sqlite").expect("write junk");
    let _scratch = Scratch(path.clone());
    let mut port = DetectingPort::new();
    let err = port.open(&path, None).expect_err("junk must be rejected");
    match err {
        PortError::OpenFailed { detail } => {
            assert!(detail.contains("unrecognized"), "{detail}");
            assert!(detail.contains(&path.display().to_string()), "{detail}");
        }
        other => panic!("expected OpenFailed, got {other:?}"),
    }
}

#[test]
fn rejects_an_empty_file_without_panicking() {
    let path = scratch_path("empty.bin");
    std::fs::write(&path, b"").expect("write empty file");
    let _scratch = Scratch(path.clone());
    let mut port = DetectingPort::new();
    let err = port
        .open(&path, None)
        .expect_err("an empty file has no magic");
    assert!(matches!(err, PortError::OpenFailed { .. }), "{err:?}");
}

#[test]
fn unopened_port_reports_reads_as_unsupported() {
    let port = DetectingPort::new();
    assert!(port.tables().is_empty());
    let err = port
        .read_rows("notes", RowRange { start: 0, len: 1 })
        .expect_err("nothing is open");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn sqlite_writes_relationships_and_close_delegate_through_detection() {
    let path = scratch_path("delegate.sqlite");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE parents (id INTEGER PRIMARY KEY, label TEXT);
         CREATE TABLE children (id INTEGER PRIMARY KEY,
             parent INTEGER REFERENCES parents);",
    )
    .expect("populate the scratch database");
    let _scratch = Scratch(path.clone());

    let mut port = DetectingPort::new();
    port.open(&path, None).expect("open SQLite");
    let relationships = port.relationships().expect("relationships delegate");
    assert_eq!(relationships.len(), 1);

    let def = TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
        ],
    );
    port.create_table(&def).expect("create delegates");
    port.insert_row(
        "people",
        &[FieldValue::Null, FieldValue::Text("Ada".into())],
    )
    .expect("insert delegates");
    let rows = port
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read delegates");
    assert_eq!(rows.len(), 1);
    assert_eq!(port.drop_table("people").expect("drop delegates"), "people");

    port.close();
    assert!(port.tables().is_empty());
    assert!(
        port.relationships()
            .expect("closed relationships")
            .is_empty()
    );
    let err = port
        .read_rows("people", RowRange { start: 0, len: 1 })
        .expect_err("closed port has no database");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn jet_writes_delegate_through_detection_on_a_copy() {
    // Writes land on a private copy — the pinned fixture stays immutable.
    let path = scratch_path("jet-copy.mdb");
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    let _scratch = Scratch(path.clone());

    let mut port = DetectingPort::new();
    port.open(&path, None).expect("open the Jet copy");
    let err = port
        .insert_row("Table1", &[FieldValue::Null])
        .expect_err("arity mismatch is a write failure before touching the file");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");

    let def = TableDef::new("Written", vec![FieldDef::new("y", FieldKind::Memo)]);
    port.create_table(&def)
        .expect("create delegates to the Jet writer");
    assert!(port.tables().iter().any(|t| t.name == "Written"));
    let dropped = port.drop_table("Written").expect("drop delegates to Jet");
    assert_eq!(dropped, "Written");
    assert!(!port.tables().iter().any(|t| t.name == "Written"));
    assert!(port.relationships().is_ok());
}

#[test]
fn write_operations_require_an_open_database() {
    let mut port = DetectingPort::new();
    let def = TableDef::new("x", vec![FieldDef::new("y", FieldKind::Memo)]);
    assert!(matches!(
        port.create_table(&def),
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        port.insert_row("x", &[FieldValue::Null]),
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        port.drop_table("x"),
        Err(PortError::Unsupported { .. })
    ));
}

#[test]
fn a_failed_open_keeps_the_previous_database() {
    let path = scratch_path("junk-after-open.bin");
    std::fs::write(&path, b"not a database").expect("write junk");
    let _scratch = Scratch(path.clone());

    let mut port = DetectingPort::new();
    port.open(&fixture(), None).expect("open Jet4");
    assert!(port.open(&path, None).is_err());
    assert!(
        !port.tables().is_empty(),
        "the previous backend stays intact"
    );
    port.read_rows("Table1", RowRange { start: 0, len: 1 })
        .expect("the previous database remains usable");
}
