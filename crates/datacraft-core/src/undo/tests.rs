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
        source: None,
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
            Event::Failed { reason, .. } => Some(reason.clone()),
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

/// Redo replays the stored redo image and moves the entry back onto the
/// undo stack — the second half of the history cycle.
#[test]
fn a_redo_rewrites_the_row_and_moves_the_entry_back() {
    let mut core = notes_core();
    core.dispatch(Command::InsertRow {
        table: "Notes".to_owned(),
        values: vec![FieldValue::Text("alpha".into())],
    });
    core.dispatch(Command::Undo);
    assert_eq!(core.inspect().undo_depth, 0);
    assert_eq!(core.inspect().redo_depth, 1);
    let events = core.dispatch(Command::Redo);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Redone { .. })),
        "{events:?}"
    );
    assert_eq!(core.inspect().undo_depth, 1);
    assert_eq!(core.inspect().redo_depth, 0);
}

/// Both stacks trim at MAX_HISTORY: the oldest entry falls off.
#[test]
fn history_trims_at_the_max_depth() {
    let mut core = notes_core();
    for index in 0..(super::MAX_HISTORY + 2) {
        core.dispatch(Command::InsertRow {
            table: "Notes".to_owned(),
            values: vec![FieldValue::Text(format!("n{index}"))],
        });
    }
    assert_eq!(
        core.inspect().undo_depth,
        super::MAX_HISTORY,
        "the undo stack never grows past MAX_HISTORY"
    );
    // Undo everything that survived, then replay it: the redo→undo move
    // trims its own stack the same way.
    for _ in 0..super::MAX_HISTORY {
        core.dispatch(Command::Undo);
    }
    assert_eq!(core.inspect().redo_depth, super::MAX_HISTORY);
    for _ in 0..super::MAX_HISTORY {
        core.dispatch(Command::Redo);
    }
    assert_eq!(core.inspect().undo_depth, super::MAX_HISTORY);
}

/// A history operation that cannot apply keeps its entry retryable —
/// the stack did not change, so the next attempt retries the same image.
/// The redo direction re-pushes onto the redo stack the same way.
#[test]
fn a_refused_redo_keeps_its_entry_retryable() {
    let mut core = notes_core();
    core.state
        .database
        .as_mut()
        .expect("database open")
        .redo
        .push(HistoryEntry {
            table: "Notes".to_owned(),
            undo: Vec::new(),
            redo: vec![HistoryOp::Update {
                from: vec![FieldValue::Text("ghost".into())],
                to: vec![FieldValue::Text("other".into())],
            }],
        });
    let reason = failure(&core.dispatch(Command::Redo));
    assert!(reason.contains("stale"), "{reason}");
    assert_eq!(
        core.inspect().redo_depth,
        1,
        "the refused entry stays on the redo stack for a retry"
    );
}
