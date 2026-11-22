//! SQL dialect selection over the composed stack (phase 98, step 98.1;
//! item M129): SQLite runs natively, Jet stays on the shadow engine, and
//! a native write is a capability refusal that leaves the file untouched.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{SqlDialect, StoragePort, Support};
use datacraft_storage::detecting::DetectingPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sql-dialect-{}-{n}-{stem}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn sqlite_core() -> (Core, Scratch) {
    let path = scratch_path("native");
    let conn = rusqlite::Connection::open(&path).expect("scratch");
    conn.execute_batch(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
         INSERT INTO items (name) VALUES ('Ada'), ('Grace');",
    )
    .expect("schema");
    drop(conn);
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path,
        password: None,
        source: None,
    });
    assert!(matches!(events.first(), Some(Event::DatabaseOpened { .. })));
    (core, scratch)
}

#[test]
fn a_sqlite_source_names_native_and_runs_the_full_dialect() {
    let (mut core, _scratch) = sqlite_core();
    let database = core.inspect().database.expect("open");
    assert_eq!(database.sql_dialect, SqlDialect::Native);
    assert_eq!(database.capabilities.native_sql, Support::Yes);

    let events = core.dispatch(Command::RunSql {
        sql: "SELECT ROW_NUMBER() OVER (ORDER BY id) AS n, name FROM items ORDER BY id".into(),
    });
    assert!(
        matches!(events.as_slice(), [Event::SqlRan { rows: 2 }]),
        "{events:?}"
    );
    let result = core.sql_result().expect("result");
    assert_eq!(result.columns, vec!["n", "name"]);
    assert_eq!(result.rows.len(), 2);
}

#[test]
fn a_native_write_is_a_capability_refusal_and_changes_nothing() {
    let (mut core, scratch) = sqlite_core();
    let events = core.dispatch(Command::RunSql {
        sql: "DELETE FROM items".into(),
    });
    match events.as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Unsupported);
            assert!(reason.contains("command bus"), "{reason}");
        }
        other => panic!("expected a capability refusal, got {other:?}"),
    }
    assert!(core.sql_result().is_none());
    let mut check = DetectingPort::new();
    check.open(&scratch.0, None).expect("reopen");
    let result = check
        .run_native_sql("SELECT COUNT(*) FROM items")
        .expect("count");
    assert_eq!(
        result.rows[0][0],
        datacraft_core::schema::FieldValue::Integer(2),
        "the refused write touched nothing"
    );
}

#[test]
fn a_jet_source_stays_on_the_shadow_engine() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: fixture,
        password: None,
        source: None,
    });
    assert!(matches!(events.first(), Some(Event::DatabaseOpened { .. })));
    assert_eq!(
        core.inspect().database.expect("open").sql_dialect,
        SqlDialect::Shadow
    );
    let events = core.dispatch(Command::RunSql {
        sql: "SELECT 1".into(),
    });
    assert!(
        matches!(events.as_slice(), [Event::SqlRan { rows: 1 }]),
        "the shadow engine still answers: {events:?}"
    );
}
