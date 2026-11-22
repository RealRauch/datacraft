//! Read-only native SQL pushdown (phase 98, step 98.1; item M129).
//!
//! The SQLite backend runs the user's statement on its own engine: full
//! dialect (window functions), real indexes and native views are visible,
//! `EXPLAIN QUERY PLAN` answers the plan, and every write — including
//! `WITH … DELETE` and PRAGMA writes — refuses precisely without
//! touching a row.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-pushdown-{}-{n}-{stem}.sqlite",
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
        "CREATE TABLE items (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL,
             price REAL,
             note TEXT,
             payload BLOB
         );
         CREATE INDEX items_name ON items (name);
         CREATE VIEW named_items AS SELECT name FROM items;
         INSERT INTO items (id, name, price, note, payload)
             VALUES (1, 'Ada', 12.5, 'hello', x'0102');
         INSERT INTO items (id, name, price, note, payload)
             VALUES (2, 'Grace', 4.25, NULL, NULL);",
    )
    .expect("schema and rows");
    drop(conn);
    Scratch(path)
}

fn open(path: &std::path::Path) -> SqlitePort {
    let mut port = SqlitePort::new();
    let opened = port.open(path, None).expect("open");
    assert_eq!(opened.format, DatabaseFormat::Sqlite);
    assert_eq!(
        port.capabilities().native_sql,
        datacraft_core::port::Support::Yes
    );
    port
}

#[test]
fn the_full_dialect_runs_natively() {
    let scratch = database("window");
    let port = open(&scratch.0);
    let result = port
        .run_native_sql("SELECT ROW_NUMBER() OVER (ORDER BY id) AS n, name FROM items ORDER BY id")
        .expect("window function");
    assert_eq!(result.columns, vec!["n", "name"]);
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
        ]
    );
}

#[test]
fn typed_values_and_native_views_are_visible() {
    let scratch = database("types");
    let port = open(&scratch.0);
    let result = port
        .run_native_sql("SELECT id, price, note, payload FROM items ORDER BY id")
        .expect("typed select");
    assert_eq!(result.rows[0][0], FieldValue::Integer(1));
    assert_eq!(result.rows[0][1], FieldValue::Float(12.5));
    assert_eq!(result.rows[0][2], FieldValue::Text("hello".into()));
    assert_eq!(result.rows[0][3], FieldValue::Binary(vec![1, 2]));
    assert_eq!(result.rows[1][2], FieldValue::Null);

    let view = port
        .run_native_sql("SELECT name FROM named_items ORDER BY name")
        .expect("native view");
    assert_eq!(
        view.rows,
        vec![
            vec![FieldValue::Text("Ada".into())],
            vec![FieldValue::Text("Grace".into())],
        ]
    );
}

#[test]
fn explain_query_plan_answers_the_plan_with_index_use() {
    let scratch = database("plan");
    let port = open(&scratch.0);
    let result = port
        .run_native_sql("EXPLAIN QUERY PLAN SELECT * FROM items WHERE name = 'Ada'")
        .expect("plan");
    assert!(
        result.columns.iter().any(|column| column == "detail"),
        "{:?}",
        result.columns
    );
    let detail = result
        .rows
        .iter()
        .filter_map(|row| row.last())
        .filter_map(|value| match value {
            FieldValue::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        detail.contains("USING INDEX"),
        "the plan names the index: {detail}"
    );
    assert!(detail.contains("items_name"), "{detail}");
}

#[test]
fn every_write_statement_is_refused_and_changes_nothing() {
    let scratch = database("writes");
    let port = open(&scratch.0);
    let writes = [
        "DELETE FROM items",
        "UPDATE items SET name = 'x'",
        "INSERT INTO items (name) VALUES ('x')",
        "WITH doomed AS (SELECT id FROM items) DELETE FROM items WHERE id IN doomed",
        "PRAGMA user_version = 7",
        "CREATE TABLE extra (id INTEGER)",
    ];
    for statement in writes {
        let error = port
            .run_native_sql(statement)
            .expect_err("the statement must be refused");
        assert!(
            matches!(error, PortError::Unsupported { .. }),
            "{statement:?}: {error:?}"
        );
    }
    let count = port
        .run_native_sql("SELECT COUNT(*) FROM items")
        .expect("count");
    assert_eq!(count.rows[0][0], FieldValue::Integer(2), "no row changed");
    let version = port
        .run_native_sql("PRAGMA user_version")
        .expect("pragma read");
    assert_eq!(
        version.rows[0][0],
        FieldValue::Integer(0),
        "no pragma write"
    );
}

#[test]
fn malformed_empty_and_multi_statements_fail_precisely() {
    let scratch = database("errors");
    let port = open(&scratch.0);
    for statement in ["", "   ", "SELECT FROM", "SELECT 1; SELECT 2"] {
        let error = port
            .run_native_sql(statement)
            .expect_err("the statement must fail");
        assert!(
            matches!(error, PortError::QueryFailed { .. }),
            "{statement:?}: {error:?}"
        );
    }
    // The trailing separator of one statement is tolerated.
    let ok = port.run_native_sql("SELECT 1;").expect("one statement");
    assert_eq!(ok.rows, vec![vec![FieldValue::Integer(1)]]);
}
