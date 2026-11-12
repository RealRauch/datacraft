//! Foreign SQLite views as read-only catalog objects (phase 73, step 73.1;
//! item M90).
//!
//! A view that exists in the file itself (created by another tool or an
//! earlier session) is `sqlite_master type='view'`: DataCraft lists it with
//! its columns, reads its rows, and answers every write with a precise
//! `Unsupported` — a view is never a DataCraft table.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{FieldValue, TableDef};
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-view-{}-{n}-{stem}-{stamp}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A file with one table and one view over it, rows included.
fn database(stem: &str) -> Scratch {
    let path = scratch_path(stem);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT, born DATETIME);
         CREATE VIEW adults AS SELECT id, name, born, name || '!' AS shout FROM people;
         INSERT INTO people (name, born) VALUES ('Ada', '2020-01-02'), ('Grace', '2021-03-04');",
    )
    .expect("populate scratch database");
    Scratch(path)
}

fn opened(stem: &str) -> (SqlitePort, Scratch) {
    let scratch = database(stem);
    let mut port = SqlitePort::new();
    port.open(&scratch.0, None).expect("open the database");
    (port, scratch)
}

#[test]
fn a_foreign_view_is_listed_with_its_columns() {
    let (port, _scratch) = opened("list");
    let views = port.native_views().expect("view inventory");
    assert_eq!(views.len(), 1, "{views:?}");
    let view = &views[0];
    assert_eq!(view.name, "adults");
    assert!(view.sql.contains("CREATE VIEW"), "{:?}", view.sql);
    let columns: Vec<&str> = view
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(columns, ["id", "name", "born", "shout"]);
    // The view never becomes a writable table.
    let catalog = port.tables();
    let tables: Vec<&str> = catalog.iter().map(|table| table.name.as_str()).collect();
    assert_eq!(tables, ["people"]);
}

#[test]
fn a_foreign_view_reads_its_rows() {
    let (port, _scratch) = opened("read");
    let rows = port
        .read_rows(
            "adults",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("view rows read");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][3], FieldValue::Text("Ada!".into()));
}

#[test]
fn view_writes_answer_a_precise_unsupported() {
    let (mut port, _scratch) = opened("writes");
    let insert = port
        .insert_row(
            "adults",
            &[
                FieldValue::Integer(3),
                FieldValue::Text("Lin".into()),
                FieldValue::Null,
                FieldValue::Text("Lin!".into()),
            ],
        )
        .expect_err("insert into a view is refused");
    assert!(
        matches!(insert, PortError::Unsupported { .. }),
        "{insert:?}"
    );
    assert!(insert.to_string().contains("read-only"), "{insert}");

    let update = port
        .update_row(
            "adults",
            0,
            &[
                FieldValue::Integer(1),
                FieldValue::Text("X".into()),
                FieldValue::Null,
                FieldValue::Text("X!".into()),
            ],
        )
        .expect_err("update through a view is refused");
    assert!(update.to_string().contains("read-only"), "{update}");
    let delete = port
        .delete_row("adults", 0)
        .expect_err("delete through a view is refused");
    assert!(delete.to_string().contains("read-only"), "{delete}");
}

/// A DataCraft-created (sidecar-applied) native view is in the same
/// inventory; the core filters it against the sidecar authority — the port
/// reports what the file holds.
#[test]
fn created_native_views_join_the_inventory() {
    let (mut port, _scratch) = opened("created");
    port.create_native_view("adults2", "SELECT id FROM people")
        .expect("create native view");
    let views = port.native_views().expect("inventory");
    let names: Vec<&str> = views.iter().map(|view| view.name.as_str()).collect();
    assert!(names.contains(&"adults2"), "{names:?}");
    assert_eq!(
        views
            .iter()
            .find(|v| v.name == "adults2")
            .map(|v| v.fields.len()),
        Some(1)
    );
    assert!(
        !port
            .tables()
            .iter()
            .any(|table: &TableDef| table.name == "adults2")
    );
}
