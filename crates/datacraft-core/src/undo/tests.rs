//! History error paths (step 28.1, item U14): stale images, missing
//! tables, read-only sources and the degraded insert record.

use crate::command::Command;
use crate::core::Core;
use crate::event::Event;
use crate::port::{DriverCapabilities, MockPort};
use crate::schema::{FieldDef, FieldKind, FieldValue, TableDef};

use super::{HistoryEntry, HistoryOp};

fn notes() -> TableDef {
    TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    )
}

fn core_with(port: MockPort) -> Core {
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "notes.mdb".into(),
        password: None,
    });
    core
}

fn notes_core() -> Core {
    core_with(MockPort::with_tables(vec![notes()]))
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

fn push(core: &mut Core, entry: HistoryEntry) {
    core.state
        .database
        .as_mut()
        .expect("database open")
        .undo
        .push(entry);
}

#[test]
fn read_only_sources_refuse_history_operations() {
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    let mut port = MockPort::with_tables(vec![notes()]);
    port.with_capabilities(caps);
    let mut core = core_with(port);
    for command in [Command::Undo, Command::Redo] {
        let reason = failure(&core.dispatch(command));
        assert!(reason.contains("cannot write rows"), "{reason}");
    }
}

#[test]
fn a_stale_delete_entry_is_reported_and_kept() {
    let mut core = notes_core();
    push(
        &mut core,
        HistoryEntry {
            table: "Notes".to_owned(),
            undo: vec![HistoryOp::Delete {
                values: vec![FieldValue::Text("ghost".into())],
            }],
            redo: Vec::new(),
        },
    );
    let reason = failure(&core.dispatch(Command::Undo));
    assert!(reason.contains("stale"), "{reason}");
    assert_eq!(core.inspect().undo_depth, 1, "the entry stays retryable");
}

#[test]
fn a_stale_update_entry_is_reported() {
    let mut core = notes_core();
    push(
        &mut core,
        HistoryEntry {
            table: "Notes".to_owned(),
            undo: vec![HistoryOp::Update {
                from: vec![FieldValue::Text("ghost".into())],
                to: vec![FieldValue::Text("other".into())],
            }],
            redo: Vec::new(),
        },
    );
    let reason = failure(&core.dispatch(Command::Undo));
    assert!(reason.contains("stale"), "{reason}");
}

#[test]
fn an_entry_for_a_missing_table_is_reported() {
    let mut core = notes_core();
    push(
        &mut core,
        HistoryEntry {
            table: "Ghost".to_owned(),
            undo: vec![HistoryOp::Restore {
                values: vec![FieldValue::Text("x".into())],
            }],
            redo: Vec::new(),
        },
    );
    let reason = failure(&core.dispatch(Command::Undo));
    assert!(reason.contains("Ghost"), "{reason}");
}

#[test]
fn a_failed_read_back_clears_the_insert_history() {
    let mut port = MockPort::with_tables(vec![notes()]);
    port.with_read_failure("read blocked");
    let mut core = core_with(port);
    let events = core.dispatch(Command::InsertRow {
        table: "Notes".to_owned(),
        values: vec![FieldValue::Text("alpha".into())],
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::RowInserted { .. })),
        "{events:?}"
    );
    assert_eq!(
        core.inspect().undo_depth,
        0,
        "without the read-back image the history degrades to empty"
    );
}
