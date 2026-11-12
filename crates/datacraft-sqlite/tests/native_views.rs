//! Native SQLite view contracts (step 71.3, item M93).
//!
//! SQLite compiles sidecar views to `CREATE VIEW`; a failing native create
//! rolls the sidecar push back, so the two authorities never diverge.

use std::path::{Path, PathBuf};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::view::ViewCommand;
use datacraft_sqlite::SqlitePort;

fn scratch_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "datacraft-native-views-{}-{tag}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join("views.sqlite")
}

fn seed(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("create file");
    conn.execute_batch("CREATE TABLE Notes (id INTEGER PRIMARY KEY, body TEXT);")
        .expect("seed table");
}

fn core_over(path: &Path) -> Core {
    let mut core = Core::new(Box::new(SqlitePort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: path.to_path_buf(),
        password: None,
        source: None,
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    core
}

fn native_view_count(path: &Path, name: &str) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("open for check");
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'view' AND name = ?1",
        [name],
        |row| row.get(0),
    )
    .expect("count")
}

/// Create applies the view natively; delete drops it.
#[test]
fn sqlite_applies_and_drops_views_natively() {
    let path = scratch_path("apply");
    seed(&path);
    let mut core = core_over(&path);

    let events = core.dispatch(Command::View(ViewCommand::Create {
        name: "AllNotes".into(),
        sql: "SELECT * FROM Notes".into(),
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::ViewCreated { .. })),
        "{events:?}"
    );
    assert_eq!(native_view_count(&path, "AllNotes"), 1);

    let events = core.dispatch(Command::View(ViewCommand::Delete {
        name: "AllNotes".into(),
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::ViewDeleted { .. })),
        "{events:?}"
    );
    assert_eq!(native_view_count(&path, "AllNotes"), 0);
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
}

/// A failing native create (an existing view of the same name) rolls the
/// sidecar push back and keeps the native definition.
#[test]
fn a_failing_native_view_rolls_the_sidecar_back() {
    let path = scratch_path("rollback");
    seed(&path);
    let conn = rusqlite::Connection::open(&path).expect("open for seed");
    conn.execute_batch("CREATE VIEW \"AllNotes\" AS SELECT 1;")
        .expect("seed native view");
    drop(conn);

    let mut core = core_over(&path);
    let events = core.dispatch(Command::View(ViewCommand::Create {
        name: "AllNotes".into(),
        sql: "SELECT * FROM Notes".into(),
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::Failed { .. })),
        "the native collision fails: {events:?}"
    );
    let db = core.inspect().database.expect("open");
    assert!(
        db.views.is_empty(),
        "the sidecar push was rolled back: {:?}",
        db.views
    );
    assert_eq!(
        native_view_count(&path, "AllNotes"),
        1,
        "the earlier native view stays"
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
}
