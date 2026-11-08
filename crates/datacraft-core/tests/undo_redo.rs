//! Row-write undo/redo through the bus (step 28.1, item U14).
//!
//! The history is per database, bounded, and records row writes only:
//! inserts, updates and deletes round-trip through `Undo`/`Redo`; a new
//! write clears the redo stack; journal batches and schema changes are
//! barriers that clear the history instead of replaying stale row images.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::journal::WriteOp;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

fn all_rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows(
        "Notes",
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read Notes")
}

fn insert(core: &mut Core, body: &str) -> Vec<Event> {
    core.dispatch(Command::InsertRow {
        table: "Notes".to_owned(),
        values: vec![FieldValue::Text(body.to_owned())],
    })
}

fn core() -> Core {
    let notes = TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![notes])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("notes.mdb"),
        password: None,
    });
    core
}

fn failure_reason(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected Failed: {events:?}"))
}

#[test]
fn insert_update_and_delete_undo_and_redo() {
    let mut core = core();
    insert(&mut core, "alpha");
    assert_eq!(core.inspect().undo_depth, 1);
    assert_eq!(core.inspect().redo_depth, 0);

    let events = core.dispatch(Command::Undo);
    assert!(
        matches!(events.first(), Some(Event::Undone { table }) if table == "Notes"),
        "{events:?}"
    );
    assert!(all_rows(&core).is_empty());
    assert_eq!(core.inspect().undo_depth, 0);
    assert_eq!(core.inspect().redo_depth, 1);

    let events = core.dispatch(Command::Redo);
    assert!(
        matches!(events.first(), Some(Event::Redone { table }) if table == "Notes"),
        "{events:?}"
    );
    assert_eq!(
        all_rows(&core),
        vec![vec![FieldValue::Text("alpha".into())]]
    );

    core.dispatch(Command::UpdateRow {
        table: "Notes".to_owned(),
        row: 0,
        values: vec![FieldValue::Text("beta".to_owned())],
    });
    core.dispatch(Command::Undo);
    assert_eq!(
        all_rows(&core),
        vec![vec![FieldValue::Text("alpha".into())]]
    );
    core.dispatch(Command::Redo);
    assert_eq!(all_rows(&core), vec![vec![FieldValue::Text("beta".into())]]);

    core.dispatch(Command::DeleteRow {
        table: "Notes".to_owned(),
        row: 0,
    });
    assert!(all_rows(&core).is_empty());
    core.dispatch(Command::Undo);
    assert_eq!(all_rows(&core), vec![vec![FieldValue::Text("beta".into())]]);
    core.dispatch(Command::Redo);
    assert!(all_rows(&core).is_empty());
}

#[test]
fn a_new_write_clears_the_redo_stack() {
    let mut core = core();
    insert(&mut core, "alpha");
    core.dispatch(Command::Undo);
    assert_eq!(core.inspect().redo_depth, 1);

    insert(&mut core, "beta");
    assert_eq!(core.inspect().redo_depth, 0);
    let reason = failure_reason(&core.dispatch(Command::Redo));
    assert!(reason.contains("nothing to redo"), "{reason}");
}

#[test]
fn the_history_is_bounded() {
    let mut core = core();
    for index in 0..(datacraft_core::undo::MAX_HISTORY + 5) {
        insert(&mut core, &format!("row{index}"));
    }
    assert_eq!(core.inspect().undo_depth, datacraft_core::undo::MAX_HISTORY);
    for _ in 0..datacraft_core::undo::MAX_HISTORY {
        let events = core.dispatch(Command::Undo);
        assert!(!events.iter().any(|e| matches!(e, Event::Failed { .. })));
    }
    let reason = failure_reason(&core.dispatch(Command::Undo));
    assert!(reason.contains("nothing to undo"), "{reason}");
}

#[test]
fn journal_batches_are_a_history_barrier() {
    let mut core = core();
    insert(&mut core, "alpha");
    let events = core.dispatch(Command::PreviewWrite {
        table: "Notes".to_owned(),
        ops: vec![WriteOp::Insert {
            values: vec![FieldValue::Text("beta".to_owned())],
        }],
    });
    let token = events
        .iter()
        .find_map(|event| match event {
            Event::WritePreviewed { token, .. } => Some(*token),
            _ => None,
        })
        .expect("preview token");
    core.dispatch(Command::ApplyWrite { token });
    assert_eq!(core.inspect().undo_depth, 0);
    let reason = failure_reason(&core.dispatch(Command::Undo));
    assert!(reason.contains("nothing to undo"), "{reason}");
}

#[test]
fn undo_and_redo_need_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let reason = failure_reason(&core.dispatch(Command::Undo));
    assert!(reason.contains("no database"), "{reason}");
    let reason = failure_reason(&core.dispatch(Command::Redo));
    assert!(reason.contains("no database"), "{reason}");
}

/// L15 regression: a barrier command that failed changed nothing, so it
/// must not erase the still-valid undo history.
#[test]
fn failed_barrier_commands_keep_the_undo_history() {
    let mut core = core();
    insert(&mut core, "first");
    assert_eq!(core.inspect().undo_depth, 1);

    let duplicate = TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    );
    let events = core.dispatch(Command::CreateTable {
        definition: duplicate,
    });
    assert!(failure_reason(&events).contains("duplicate"), "{events:?}");
    assert_eq!(
        core.inspect().undo_depth,
        1,
        "a failed table creation must not clear the undo stack"
    );
}
