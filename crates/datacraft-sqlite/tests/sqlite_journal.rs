//! Journal batches on SQLite (step 85.3, item H61; retired M20 refusal).
//!
//! SQLite applies a `write_row_ops` batch inside one explicit
//! transaction — either every operation lands or none does — and
//! reports `atomic_batch: true`, so the datasheet writes through the
//! pending-edit batch on SQLite like on Jet. The M20 one-by-one refusal
//! is retired by H61; the Jet atomic path stays pinned by the core
//! journal suite and `jet_row_writes.rs`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::WriteOp;
use datacraft_core::port::{PortError, RowRange, RowWriteOp, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_sqlite::SqlitePort;

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-journal-{}-{n}-{stamp}.sqlite",
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
        source: None,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    core
}

fn rows(core: &Core) -> Vec<String> {
    core.read_rows(
        "notes",
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read")
    .into_iter()
    .map(|row| match &row[0] {
        FieldValue::Text(text) => text.clone(),
        other => panic!("text column, got {other:?}"),
    })
    .collect()
}

/// A multi-operation batch previews, applies as one atomic write and
/// reports the capability (step 85.3, item H61).
#[test]
fn sqlite_applies_multi_op_batches_atomically() {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let mut core = open_core(&path);
    let db = core.inspect().database.expect("database open");
    assert!(
        db.capabilities.atomic_batch,
        "SQLite applies batches atomically (H61)"
    );

    let events = core.dispatch(Command::PreviewWrite {
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
            WriteOp::Delete { row: 1 },
            WriteOp::Insert {
                values: vec![FieldValue::Text("gamma".into())],
            },
        ],
    });
    let token = events
        .iter()
        .find_map(|event| match event {
            Event::WritePreviewed { token, .. } => Some(*token),
            _ => None,
        })
        .expect("multi-op preview");
    let events = core.dispatch(Command::ApplyWrite { token });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::WriteApplied { .. })),
        "{events:?}"
    );
    assert_eq!(rows(&core), vec!["one", "gamma"]);
    drop(scratch);
}

/// A failing batch rolls back: the valid operations before the failure
/// leave no partial write behind (port-level proof).
#[test]
fn a_failed_sqlite_batch_rolls_back_every_operation() {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let mut port = SqlitePort::new();
    port.open(&path, None).expect("open scratch");
    let error = port
        .write_row_ops(
            "notes",
            &[
                RowWriteOp::Insert(vec![FieldValue::Text("gamma".into())]),
                RowWriteOp::Update {
                    row: 99,
                    values: vec![FieldValue::Text("nope".into())],
                },
            ],
        )
        .expect_err("the out-of-range update fails the batch");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");
    let bodies: Vec<String> = port
        .read_rows(
            "notes",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read")
        .into_iter()
        .map(|row| match &row[0] {
            FieldValue::Text(text) => text.clone(),
            other => panic!("text column, got {other:?}"),
        })
        .collect();
    assert_eq!(
        bodies,
        vec!["alpha", "beta"],
        "the rolled-back insert left no partial write"
    );
    // The connection stays usable after the rolled-back batch (M25).
    port.write_row_ops(
        "notes",
        &[RowWriteOp::Insert(vec![FieldValue::Text("delta".into())])],
    )
    .expect("single-op batch after rollback");
    assert_eq!(
        bodies.len() + 1,
        port.read_rows(
            "notes",
            RowRange {
                start: 0,
                len: usize::MAX,
            }
        )
        .expect("read")
        .len()
    );
    drop(scratch);
}
