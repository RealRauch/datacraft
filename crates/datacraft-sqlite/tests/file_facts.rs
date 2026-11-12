//! SQLite file facts and the bounded health check (phase 73, step 73.3;
//! item M91).
//!
//! `PRAGMA` facts (version, page size, journal mode, encoding, user
//! version) ride the open; `quick_check(1)` reports a corrupt file as a
//! precise health result instead of a late, unrelated query error.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::StoragePort;
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-facts-{}-{n}-{stem}-{stamp}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn database(stem: &str) -> Scratch {
    let path = scratch_path(stem);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "PRAGMA user_version = 7;
         CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);
         INSERT INTO notes (body) VALUES ('hello');",
    )
    .expect("populate scratch database");
    Scratch(path)
}

fn value<'a>(facts: &'a [datacraft_core::port::SourceFact], name: &str) -> Option<&'a str> {
    facts
        .iter()
        .find(|fact| fact.name == name)
        .map(|fact| fact.value.as_str())
}

#[test]
fn the_open_reports_the_file_facts() {
    let scratch = database("facts");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    let report = port.source_facts();
    assert_eq!(value(&report.facts, "page_size"), Some("4096"));
    assert_eq!(value(&report.facts, "journal_mode"), Some("delete"));
    assert_eq!(value(&report.facts, "encoding"), Some("UTF-8"));
    assert_eq!(value(&report.facts, "user_version"), Some("7"));
    assert!(
        value(&report.facts, "sqlite_version").is_some_and(|v| v.starts_with('3')),
        "{:?}",
        report.facts
    );
    assert_eq!(report.health, None, "a fresh database passes quick_check");
}

/// A malformed data page passes the catalog read but fails
/// `quick_check(1)` — the open reports the precise health reason.
#[test]
fn a_corrupt_file_reports_a_precise_health_reason() {
    let path = scratch_path("corrupt");
    let scratch = Scratch(path.clone());
    let conn = rusqlite::Connection::open(&path).expect("create");
    conn.execute_batch(
        "PRAGMA journal_mode = DELETE;
         CREATE TABLE big (id INTEGER PRIMARY KEY, payload TEXT);",
    )
    .expect("create table");
    let payload = "x".repeat(200);
    for _ in 0..2000 {
        conn.execute("INSERT INTO big (payload) VALUES (?1)", [&payload])
            .expect("insert");
    }
    drop(conn);
    let size = std::fs::metadata(&path).expect("metadata").len() as usize;
    // Corrupt the last page (a data page) while the catalog stays readable.
    let mut bytes = std::fs::read(&path).expect("read");
    let last = size - 4096;
    bytes[last..].fill(0xFF);
    std::fs::write(&path, &bytes).expect("write corrupt copy");

    let mut port = SqlitePort::new();
    port.open(&scratch.0, None)
        .expect("the catalog still opens");
    let report = port.source_facts();
    let health = report.health.expect("quick_check reports the damage");
    assert!(health.contains("quick_check"), "{health}");
}
