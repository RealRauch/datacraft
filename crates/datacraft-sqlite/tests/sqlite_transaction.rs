//! SQLite transaction hygiene on failed DDL (step 34.5, item M25).
//!
//! `create_table` runs the table plus its indexes in one transaction. A
//! failure after the CREATE TABLE must roll the transaction back: the
//! connection stays usable and no half-created table survives.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::StoragePort;
use datacraft_core::schema::{FieldDef, FieldKind, IndexMode, TableDef};
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-tx-{}-{n}-{stem}-{stamp}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn empty_database(stem: &str) -> Scratch {
    let path = scratch_path(stem);
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create empty scratch database");
    conn.execute_batch("CREATE TABLE seed (x INTEGER); DROP TABLE seed;")
        .expect("force the SQLite header");
    Scratch(path)
}

/// One table whose single field carries a unique index. The generated index
/// name is `idx_<table>_<field>`.
fn indexed_table(name: &str, field: &str) -> TableDef {
    let mut definition = TableDef::new(
        name,
        vec![FieldDef::new(field, FieldKind::Text { max_len: 20 })],
    );
    definition.fields[0].properties.indexed = IndexMode::NoDuplicates;
    definition
}

#[test]
fn a_failed_index_creation_rolls_back_and_keeps_the_connection_usable() {
    let scratch = empty_database("rollback");
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open empty database");

    port.create_table(&indexed_table("a_b", "c"))
        .expect("first table");

    // Both definitions derive the index name `idx_a_b_c`: the CREATE TABLE
    // succeeds, the CREATE INDEX fails.
    let err = port
        .create_table(&indexed_table("a", "b_c"))
        .expect_err("colliding index name must fail");
    assert!(err.to_string().contains("creating table"), "{err}");

    // The failed transaction must be closed — the next create works.
    port.create_table(&indexed_table("recovery", "id"))
        .expect("the connection stays usable after the rollback");
    assert!(port.tables().iter().any(|table| table.name == "recovery"));

    // And the half-created table must not exist anywhere.
    let conn = rusqlite::Connection::open(&scratch.0).expect("inspect the file");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'a'",
            [],
            |row| row.get(0),
        )
        .expect("count tables");
    assert_eq!(count, 0, "the failed create must not leave a table behind");
}

/// Creation refuses an existing target and a foreign format family.
#[test]
fn creation_refuses_existing_files_and_foreign_formats() {
    use datacraft_core::port::DatabaseFormat;

    let scratch = empty_database("exists");
    let mut port = SqlitePort::new();
    let err = port
        .create_database(&scratch.0, DatabaseFormat::Sqlite)
        .expect_err("an existing file must be refused");
    assert!(err.to_string().contains("already exists"), "{err}");

    let fresh = scratch_path("foreign");
    let err = port
        .create_database(&fresh, DatabaseFormat::Jet4)
        .expect_err("Jet4 is not a SQLite creation target");
    assert!(err.to_string().contains("SQLite"), "{err}");
    assert!(!fresh.exists(), "no file is created for a foreign format");
}
