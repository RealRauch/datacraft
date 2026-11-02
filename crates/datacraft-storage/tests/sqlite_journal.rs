//! Journal batches on a non-atomic driver (step 33.1, item M20).
//!
//! SQLite reports `atomic_batch: false`; the core therefore refuses
//! multi-operation previews with a precise reason and keeps
//! single-operation batches available. Jet's atomic batch path is pinned
//! by the core journal suite and `jet_row_writes.rs`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::WriteOp;
use datacraft_core::port::RowRange;
use datacraft_core::schema::FieldValue;
use datacraft_storage::sqlite::SqlitePort;

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-journal-{}-{n}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn open_core(path: &PathBuf) -> Core {
    let conn = rusqlite::Connection::open(path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE notes (body TEXT);
         INSERT INTO notes (body) VALUES ('alpha'), ('beta');",
    )
    .expect("seed the scratch database");
    drop(conn);
    let mut core = Core::new(Box::new(SqlitePort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: path.clone(),
        password: None,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    core
}

fn failure(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected Failed: {events:?}"))
}

#[test]
fn sqlite_refuses_multi_op_batches_and_keeps_single_op_ones() {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let mut core = open_core(&path);
    let db = core.inspect().database.expect("database open");
    assert!(!db.capabilities.atomic_batch, "SQLite is not atomic");

    let reason = failure(&core.dispatch(Command::PreviewWrite {
        table: "notes".to_owned(),
        ops: vec![
            WriteOp::Update {
                row: 0,
                values: vec![FieldValue::Text("one".into())],
            },
            WriteOp::Update {
                row: 1,
                values: vec![FieldValue::Text("two".into())],
            },
        ],
    }));
    assert!(
        reason.contains("atomic") && reason.contains("one operation"),
        "{reason}"
    );

    let events = core.dispatch(Command::PreviewWrite {
        table: "notes".to_owned(),
        ops: vec![WriteOp::Update {
            row: 0,
            values: vec![FieldValue::Text("single".into())],
        }],
    });
    let token = events
        .iter()
        .find_map(|event| match event {
            Event::WritePreviewed { token, .. } => Some(*token),
            _ => None,
        })
        .expect("single-op preview");
    let events = core.dispatch(Command::ApplyWrite { token });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::WriteApplied { .. })),
        "{events:?}"
    );
    let rows = core
        .read_rows(
            "notes",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read");
    assert_eq!(rows[0][0], FieldValue::Text("single".into()));
    drop(scratch);
}
