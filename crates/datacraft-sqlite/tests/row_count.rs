//! Row count without a read over SQLite (phase 101, step 101.5; item
//! M132): the driver answers `SELECT COUNT(*)` and the number agrees with
//! a full read.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{RowRange, StoragePort};
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-count-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn populated(rows: usize) -> Scratch {
    let path = scratch_path("data.sqlite");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch("CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);")
        .expect("create table");
    for index in 0..rows {
        conn.execute(
            "INSERT INTO notes (body) VALUES (?1)",
            [format!("row{index}")],
        )
        .expect("insert");
    }
    Scratch(path)
}

#[test]
fn the_count_answers_without_reading_and_agrees_with_a_full_read() {
    let database = populated(37);
    let mut port = SqlitePort::new();
    port.open(&database.0, None).expect("opens");
    assert_eq!(port.row_count("notes").expect("a count"), 37);
    let rows = port
        .read_rows(
            "notes",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("a read");
    assert_eq!(rows.len(), 37);
    // An empty table counts zero, an unknown one is refused.
    port.create_table(&datacraft_core::schema::TableDef::new(
        "empty",
        vec![datacraft_core::schema::FieldDef::new(
            "id",
            datacraft_core::schema::FieldKind::Number(
                datacraft_core::schema::NumericKind::LongInteger,
            ),
        )],
    ))
    .expect("create the empty table");
    assert_eq!(port.row_count("empty").expect("a count"), 0);
    assert!(port.row_count("missing").is_err());
}

#[test]
fn a_window_past_the_end_is_empty() {
    let database = populated(5);
    let mut port = SqlitePort::new();
    port.open(&database.0, None).expect("opens");
    let window = port
        .read_rows("notes", RowRange { start: 5, len: 10 })
        .expect("a window at the end");
    assert!(window.is_empty());
}
