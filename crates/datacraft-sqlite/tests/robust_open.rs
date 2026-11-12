//! Robust open and table-shape honesty (phase 73, step 73.4; item M92).
//!
//! A bounded `busy_timeout` waits out short writer contention instead of
//! answering a bare `SQLITE_BUSY`; a WAL file reports its journal mode;
//! `WITHOUT ROWID` tables read in primary-key order and refuse rowid
//! addressing precisely; `STRICT` tables are named in the facts.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-robust-{}-{n}-{stem}-{stamp}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        // WAL sessions leave siblings behind.
        let _ = std::fs::remove_file(self.0.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(self.0.with_extension("sqlite-shm"));
    }
}

fn database(stem: &str, schema: &str) -> Scratch {
    let path = scratch_path(stem);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(schema)
        .expect("populate scratch database");
    Scratch(path)
}

fn value<'a>(facts: &'a [datacraft_core::port::SourceFact], name: &str) -> Option<&'a str> {
    facts
        .iter()
        .find(|fact| fact.name == name)
        .map(|fact| fact.value.as_str())
}

fn all_rows(port: &SqlitePort, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("rows read")
}

/// The port sets a bounded busy timeout: a short exclusive lock held by
/// another connection is waited out instead of failing with SQLITE_BUSY.
#[test]
fn a_short_write_lock_is_waited_out() {
    let scratch = database(
        "busy",
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);",
    );
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");

    let locker = rusqlite::Connection::open(&scratch.0).expect("second connection");
    locker
        .execute_batch("BEGIN IMMEDIATE;")
        .expect("take the write lock");
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        locker.execute_batch("COMMIT;").expect("release the lock");
    });
    let outcome = port.insert_row(
        "notes",
        &[FieldValue::Null, FieldValue::Text("hello".into())],
    );
    release.join().expect("lock thread");
    outcome.expect("the write waits for the lock and succeeds");
    assert_eq!(all_rows(&port, "notes").len(), 1);
    // The bound is explicit and observable, not an implicit driver default.
    let report = port.source_facts();
    assert_eq!(
        value(&report.facts, "busy_timeout"),
        Some("5000"),
        "{:?}",
        report.facts
    );
}

/// A WAL file opens and reports its journal mode as a fact.
#[test]
fn a_wal_file_reports_its_journal_mode() {
    let scratch = database(
        "wal",
        "PRAGMA journal_mode = WAL;
         CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);",
    );
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open the WAL file");
    let report = port.source_facts();
    assert_eq!(value(&report.facts, "journal_mode"), Some("wal"));
    assert_eq!(report.health, None, "a clean WAL file passes quick_check");
}

/// `WITHOUT ROWID` tables read deterministically in primary-key order and
/// refuse rowid-based row addressing with a precise reason.
#[test]
fn without_rowid_tables_read_in_key_order_and_refuse_row_writes() {
    let scratch = database(
        "norowid",
        "CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT) WITHOUT ROWID;
         INSERT INTO kv (k, v) VALUES ('b', '2'), ('a', '1'), ('c', '3');",
    );
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    let rows = all_rows(&port, "kv");
    let keys: Vec<&FieldValue> = rows.iter().map(|row| &row[0]).collect();
    assert_eq!(
        keys,
        [
            &FieldValue::Text("a".into()),
            &FieldValue::Text("b".into()),
            &FieldValue::Text("c".into()),
        ],
        "the window follows the declared primary key"
    );

    let update = port
        .update_row(
            "kv",
            0,
            &[FieldValue::Text("a".into()), FieldValue::Text("x".into())],
        )
        .expect_err("rowid addressing is refused");
    assert!(
        matches!(update, PortError::Unsupported { .. }),
        "{update:?}"
    );
    assert!(update.to_string().contains("WITHOUT ROWID"), "{update}");
    let delete = port
        .delete_row("kv", 0)
        .expect_err("rowid addressing is refused");
    assert!(delete.to_string().contains("WITHOUT ROWID"), "{delete}");

    let report = port.source_facts();
    assert_eq!(
        value(&report.facts, "without_rowid_tables"),
        Some("kv"),
        "{:?}",
        report.facts
    );
}

/// `STRICT` tables are named in the file facts.
#[test]
fn strict_tables_are_reported_in_the_facts() {
    let scratch = database(
        "strict",
        "CREATE TABLE s (id INTEGER PRIMARY KEY, name TEXT) STRICT;",
    );
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    let report = port.source_facts();
    assert_eq!(
        value(&report.facts, "strict_tables"),
        Some("s"),
        "{:?}",
        report.facts
    );
}
