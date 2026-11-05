//! SQLite index DDL and uniqueness (step 32.2, item M15).
//!
//! Creating a table with `IndexMode` emits `CREATE [UNIQUE] INDEX`; SQLite
//! then enforces uniqueness on insert/update, a freed value can be reused,
//! and the DDL survives reopen.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, FieldValue, IndexMode, TableDef,
};
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-indexes-{}-{n}-{stem}-{stamp}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn table() -> TableDef {
    TableDef::new(
        "Notes",
        vec![
            FieldDef {
                name: "name".into(),
                kind: FieldKind::Text { max_len: 20 },
                properties: FieldProperties {
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
            FieldDef {
                name: "tag".into(),
                kind: FieldKind::Text { max_len: 20 },
                properties: FieldProperties {
                    indexed: IndexMode::DuplicatesOk,
                    ..FieldProperties::default()
                },
            },
        ],
    )
}

fn opened() -> (SqlitePort, Scratch) {
    let path = scratch_path("create");
    let scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    port.create_database(&path, DatabaseFormat::Sqlite)
        .expect("create SQLite");
    port.open(&path, None).expect("open");
    port.create_table(&table()).expect("create table");
    (port, scratch)
}

fn unique_error(error: &PortError) -> bool {
    error.to_string().to_uppercase().contains("UNIQUE")
}

#[test]
fn the_matching_ddl_is_emitted_and_enforced() {
    let (mut port, scratch) = opened();
    port.insert_row(
        "Notes",
        &[
            FieldValue::Text("alpha".into()),
            FieldValue::Text("one".into()),
        ],
    )
    .expect("first row");
    let error = port
        .insert_row(
            "Notes",
            &[
                FieldValue::Text("alpha".into()),
                FieldValue::Text("two".into()),
            ],
        )
        .expect_err("duplicate unique value");
    assert!(unique_error(&error), "{error:?}");
    port.insert_row(
        "Notes",
        &[
            FieldValue::Text("beta".into()),
            FieldValue::Text("one".into()),
        ],
    )
    .expect("the non-unique index allows duplicates");
    let error = port
        .update_row(
            "Notes",
            1,
            &[
                FieldValue::Text("alpha".into()),
                FieldValue::Text("three".into()),
            ],
        )
        .expect_err("duplicate on update");
    assert!(unique_error(&error), "{error:?}");
    port.delete_row("Notes", 0).expect("delete");
    port.insert_row(
        "Notes",
        &[
            FieldValue::Text("alpha".into()),
            FieldValue::Text("four".into()),
        ],
    )
    .expect("the freed value is reusable");
    port.close();

    let conn = rusqlite::Connection::open(&scratch.0).expect("sqlite connection");
    let indexes: Vec<String> = conn
        .prepare("SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name='Notes'")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(Result::unwrap)
        .collect();
    assert_eq!(indexes.len(), 2, "{indexes:?}");
    assert!(
        indexes
            .iter()
            .any(|sql| sql.contains("UNIQUE") && sql.contains("\"name\"")),
        "{indexes:?}"
    );
    assert!(
        indexes
            .iter()
            .any(|sql| !sql.contains("UNIQUE") && sql.contains("\"tag\"")),
        "{indexes:?}"
    );
    drop(conn);

    let mut reopened = SqlitePort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    let error = reopened
        .insert_row(
            "Notes",
            &[
                FieldValue::Text("alpha".into()),
                FieldValue::Text("five".into()),
            ],
        )
        .expect_err("uniqueness survives reopen");
    assert!(unique_error(&error), "{error:?}");
}

#[test]
fn unsupported_index_kinds_are_refused_by_the_core() {
    let path = scratch_path("memo");
    let _scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    core.dispatch(Command::OpenDatabase {
        path,
        password: None,
    });
    let mut memo = FieldDef::new("body", FieldKind::Memo);
    memo.properties.indexed = IndexMode::NoDuplicates;
    let events = core.dispatch(Command::CreateTable {
        definition: TableDef::new("Notes", vec![memo]),
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        })
        .expect("memo index rejected");
    assert!(reason.contains("indexing is not supported"), "{reason}");
}
