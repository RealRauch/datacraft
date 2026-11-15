//! The `UpdateCells` command's contract suite (step 86.3, item M101).
//!
//! A multi-row cell write is **one** bus command and **one** undo step:
//! the history entry carries every row's inverse, so one undo restores
//! the whole fill. Struct failures (unknown table, out-of-range row or
//! column) answer precisely and write nothing; `Null` edits clear the
//! cells (delete-to-NULL rides the same command).

use std::path::PathBuf;

use datacraft_core::command::{CellEdit, Command};
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DriverCapabilities, MockPort, RowRange};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn items() -> TableDef {
    TableDef::new(
        "Items",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn app_with(rows: usize) -> Core {
    let data: Vec<Vec<FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                FieldValue::Integer(index as i64),
                FieldValue::Text(format!("name{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![items()]);
    port.with_row_data("Items", data);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core
}

fn all_rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows(
        "Items",
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read Items")
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

/// One command writes several rows; one undo restores all of them —
/// a fill is a single undo step (M101).
#[test]
fn one_command_updates_several_rows_and_one_undo_restores_all() {
    let mut core = app_with(4);
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![
            CellEdit {
                row: 0,
                col: 1,
                value: FieldValue::Text("first".into()),
            },
            CellEdit {
                row: 1,
                col: 1,
                value: FieldValue::Text("second".into()),
            },
            CellEdit {
                row: 2,
                col: 1,
                value: FieldValue::Text("third".into()),
            },
        ],
    });
    assert_eq!(events.len(), 3, "every edit answers its row update");
    assert_eq!(all_rows(&core)[1][1], FieldValue::Text("second".into()));
    assert_eq!(
        core.state().database.as_ref().expect("open").undo.len(),
        1,
        "the fill records one history entry"
    );

    let events = core.dispatch(Command::Undo);
    assert_eq!(events.len(), 1, "one undo event for the whole fill");
    let rows = all_rows(&core);
    assert_eq!(rows[0][1], FieldValue::Text("name0".into()));
    assert_eq!(rows[1][1], FieldValue::Text("name1".into()));
    assert_eq!(rows[2][1], FieldValue::Text("name2".into()));

    let events = core.dispatch(Command::Redo);
    assert!(
        events
            .iter()
            .all(|event| matches!(event, Event::Redone { .. })),
        "redo re-applies the whole fill"
    );
    assert_eq!(all_rows(&core)[2][1], FieldValue::Text("third".into()));
}

/// A struct failure writes nothing: an out-of-range row answers
/// precisely and leaves the history and the rows untouched.
#[test]
fn an_out_of_range_row_answers_precisely_and_writes_nothing() {
    let mut core = app_with(2);
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![
            CellEdit {
                row: 0,
                col: 1,
                value: FieldValue::Text("ok".into()),
            },
            CellEdit {
                row: 5,
                col: 1,
                value: FieldValue::Text("never".into()),
            },
        ],
    });
    let reason = failure_reason(&events);
    assert!(
        reason.contains("row 5") && reason.contains("out of range"),
        "the failure names the row: {reason}"
    );
    assert_eq!(all_rows(&core)[0][1], FieldValue::Text("name0".into()));
    assert_eq!(
        core.state().database.as_ref().expect("open").undo.len(),
        0,
        "nothing was recorded — the write never happened"
    );
}

/// An out-of-range column answers precisely with the field arity.
#[test]
fn an_out_of_range_column_answers_precisely() {
    let mut core = app_with(2);
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 7,
            value: FieldValue::Text("x".into()),
        }],
    });
    let reason = failure_reason(&events);
    assert!(
        reason.contains("column 7") && reason.contains("2 fields"),
        "the failure names column and arity: {reason}"
    );
}

/// An unknown table answers the table-not-found failure.
#[test]
fn an_unknown_table_answers_not_found() {
    let mut core = app_with(2);
    let events = core.dispatch(Command::UpdateCells {
        table: "Missing".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 0,
            value: FieldValue::Integer(1),
        }],
    });
    let reason = failure_reason(&events);
    assert!(
        reason.contains("Missing"),
        "the failure names the table: {reason}"
    );
}

/// Delete-to-NULL rides the same command: `Null` edits clear the cells
/// (the mock keeps tri-state NULL, M24), and a read-only source refuses
/// before anything is written.
#[test]
fn null_edits_clear_cells_and_read_only_sources_refuse() {
    let mut core = app_with(2);
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![
            CellEdit {
                row: 0,
                col: 1,
                value: FieldValue::Null,
            },
            CellEdit {
                row: 1,
                col: 1,
                value: FieldValue::Null,
            },
        ],
    });
    assert_eq!(events.len(), 2);
    assert_eq!(all_rows(&core)[0][1], FieldValue::Null);

    let mut port = MockPort::with_tables(vec![items()]);
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 1,
            value: FieldValue::Null,
        }],
    });
    assert!(
        events
            .iter()
            .all(|event| matches!(event, Event::Failed { .. })),
        "a read-only source refuses: {events:?}"
    );
}

/// The guards mirror the single-row paths: no open database, an empty
/// edit list, a schema-invalid value and a port write failure all answer
/// precisely and record nothing.
#[test]
fn the_guards_answer_precisely() {
    // No open database.
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![items()])));
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 1,
            value: FieldValue::Null,
        }],
    });
    assert!(
        events
            .iter()
            .all(|event| matches!(event, Event::Failed { .. })),
    );

    // Empty edit list.
    let mut core = app_with(1);
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: Vec::new(),
    });
    let reason = failure_reason(&events);
    assert!(reason.contains("no cell edits"), "{reason}");

    // A required field refuses the cleared value (schema validation).
    let mut required = TableDef::new(
        "Items",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    );
    required.fields[0].properties.required = true;
    let def = required;
    let mut port = MockPort::with_tables(vec![def]);
    port.with_row_data("Items", vec![vec![FieldValue::Text("kept".into())]]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 0,
            value: FieldValue::Null,
        }],
    });
    let reason = failure_reason(&events);
    assert!(
        reason.contains("name"),
        "the failure names the field: {reason}"
    );
    assert_eq!(
        core.read_rows("Items", RowRange { start: 0, len: 1 },)
            .expect("read")[0][0],
        FieldValue::Text("kept".into()),
        "the refused write never reached storage"
    );

    // A port read failure answers the port reason.
    let mut port = MockPort::with_tables(vec![items()]);
    port.with_read_failure("read boom");
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![CellEdit {
            row: 0,
            col: 1,
            value: FieldValue::Null,
        }],
    });
    let reason = failure_reason(&events);
    assert!(reason.contains("read boom"), "{reason}");

    // A port write failure stops the fill with the precise reason.
    let mut port = MockPort::with_tables(vec![items()]);
    port.with_row_data(
        "Items",
        (0..3)
            .map(|index| {
                vec![
                    FieldValue::Integer(index as i64),
                    FieldValue::Text(format!("name{index}")),
                ]
            })
            .collect(),
    );
    port.with_write_failure("disk full");
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = core.dispatch(Command::UpdateCells {
        table: "Items".into(),
        edits: vec![
            CellEdit {
                row: 0,
                col: 1,
                value: FieldValue::Text("x".into()),
            },
            CellEdit {
                row: 1,
                col: 1,
                value: FieldValue::Text("y".into()),
            },
        ],
    });
    let reason = failure_reason(&events);
    assert!(reason.contains("disk full"), "{reason}");
}
