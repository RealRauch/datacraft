//! Detection contract of the composed backend (step 13.1, item H35).
//!
//! One port decides by file magic which backend serves the file, so every
//! client (GUI, MCP, CLI, E2E) can open Jet/ACE and SQLite files through
//! the same command vocabulary.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;
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
    let opened = port.open(&fixture()).expect("open the Jet4 fixture");
    assert_eq!(opened.format, DatabaseFormat::Jet4);
    assert_eq!(opened.name, "testV2000");
    assert!(!port.tables().is_empty());
}

#[test]
fn detects_a_sqlite_file_and_serves_its_rows() {
    let scratch = sqlite_database();
    let mut port = DetectingPort::new();
    let opened = port.open(&scratch.0).expect("open the SQLite file");
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
    let jet = port.open(&fixture()).expect("open Jet4");
    assert_eq!(jet.format, DatabaseFormat::Jet4);
    let sqlite = port.open(&scratch.0).expect("open SQLite");
    assert_eq!(sqlite.format, DatabaseFormat::Sqlite);
    assert_eq!(port.tables().len(), 1);
}

#[test]
fn rejects_unknown_files_with_a_precise_error() {
    let path = scratch_path("junk.bin");
    std::fs::write(&path, b"neither jet nor sqlite").expect("write junk");
    let _scratch = Scratch(path.clone());
    let mut port = DetectingPort::new();
    let err = port.open(&path).expect_err("junk must be rejected");
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
    let err = port.open(&path).expect_err("an empty file has no magic");
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
