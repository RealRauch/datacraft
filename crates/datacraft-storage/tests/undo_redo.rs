//! Undo/redo on a real Jet4 file, verified by reopening (step 28.1, U14).
//!
//! A deleted row is restored with its engine-assigned AutoNumber value:
//! `restore_row` is the deliberate exception to the "AutoNumber values are
//! engine-assigned" write rule, reserved for history restoration.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, RowRange};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue};
use datacraft_storage::port_adapter::Jet4Port;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-undo-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn table() -> datacraft_core::schema::TableDef {
    datacraft_core::schema::TableDef::new(
        "Notes",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("body", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows(
        "Notes",
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read Notes")
}

fn open(path: &Path) -> Core {
    let mut core = Core::new(Box::new(Jet4Port::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: path.to_path_buf(),
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

fn failed(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Failed { .. }))
}

#[test]
fn undo_restores_an_autonumber_row_across_reopen() {
    let path = scratch_path("restore");
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(Jet4Port::new()));
    assert!(!failed(&core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    })));
    core = open(&path);
    assert!(!failed(&core.dispatch(Command::CreateTable {
        definition: table(),
    })));
    for body in ["alpha", "beta"] {
        assert!(!failed(&core.dispatch(Command::InsertRow {
            table: "Notes".to_owned(),
            values: vec![FieldValue::Null, FieldValue::Text(body.to_owned())],
        })));
    }
    let before = rows(&core);
    assert_eq!(before.len(), 2, "{before:?}");
    assert!(matches!(before[0][0], FieldValue::Integer(_)));

    assert!(!failed(&core.dispatch(Command::DeleteRow {
        table: "Notes".to_owned(),
        row: 0,
    })));
    assert_eq!(rows(&core).len(), 1);
    assert!(!failed(&core.dispatch(Command::Undo)));
    assert_eq!(rows(&core).len(), 2, "undo restores the row");

    core.dispatch(Command::CloseDatabase);
    let reopened = open(&path);
    let after = rows(&reopened);
    assert!(
        after.contains(&before[0]),
        "the restored row keeps its AutoNumber: {after:?} vs {before:?}"
    );

    core = reopened;
    assert!(!failed(&core.dispatch(Command::Redo)));
    assert_eq!(rows(&core).len(), 1, "redo removes the restored row");
    core.dispatch(Command::CloseDatabase);
    let again = open(&path);
    assert_eq!(rows(&again).len(), 1);
    drop(scratch);
}
