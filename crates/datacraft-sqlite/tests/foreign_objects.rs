//! Foreign index/trigger inventory (phase 73, step 73.2; item M90).
//!
//! `sqlite_master` is the source: explicit and automatic indexes (name,
//! columns, uniqueness) and trigger definitions (name, table, SQL) are
//! reported read-only. DataCraft never creates or executes triggers.

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
        "datacraft-sqlite-objects-{}-{n}-{stem}-{stamp}.sqlite",
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
        "CREATE TABLE people (
             id INTEGER PRIMARY KEY,
             name TEXT UNIQUE,
             city TEXT
         );
         CREATE INDEX idx_people_city ON people(city);
         CREATE TRIGGER people_guard AFTER INSERT ON people
         BEGIN
             SELECT 1;
         END;",
    )
    .expect("populate scratch database");
    Scratch(path)
}

#[test]
fn indexes_are_listed_with_columns_and_uniqueness() {
    let scratch = database("indexes");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    let mut indexes = port.indexes().expect("index inventory");
    indexes.sort_by(|left, right| left.name.cmp(&right.name));
    assert_eq!(indexes.len(), 2, "{indexes:?}");

    let auto = indexes
        .iter()
        .find(|index| index.automatic)
        .expect("the UNIQUE constraint created an automatic index");
    assert_eq!(auto.table, "people");
    assert_eq!(auto.columns, ["name"]);
    assert!(auto.unique, "the UNIQUE constraint index is unique");

    let explicit = indexes
        .iter()
        .find(|index| index.name == "idx_people_city")
        .expect("the explicit index");
    assert!(!explicit.automatic, "{explicit:?}");
    assert_eq!(explicit.columns, ["city"]);
    assert!(!explicit.unique);
}

#[test]
fn triggers_are_listed_read_only_with_their_sql() {
    let scratch = database("triggers");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    let triggers = port.triggers().expect("trigger inventory");
    assert_eq!(triggers.len(), 1, "{triggers:?}");
    assert_eq!(triggers[0].name, "people_guard");
    assert_eq!(triggers[0].table, "people");
    assert!(
        triggers[0].sql.contains("CREATE TRIGGER"),
        "{:?}",
        triggers[0].sql
    );
    // The trigger inventory grants the read-only capability.
    assert_eq!(
        port.capabilities().triggers,
        datacraft_core::port::Support::ReadOnly
    );
}

#[test]
fn a_database_without_objects_reports_empty_inventories() {
    let path = scratch_path("empty");
    let scratch = Scratch(path.clone());
    let conn = rusqlite::Connection::open(&path).expect("create");
    conn.execute_batch("CREATE TABLE plain (id INTEGER, note TEXT);")
        .expect("populate");
    drop(conn);
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open");
    assert!(port.indexes().expect("indexes").is_empty());
    assert!(port.triggers().expect("triggers").is_empty());
}
