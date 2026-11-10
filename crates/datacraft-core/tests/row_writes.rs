//! Row update/delete command contract (step 11.10, item H40).
//!
//! Updates replace a row by its zero-based live-row index; deletes remove
//! it. The bus validates the values against the schema, preserves
//! AutoNumber values (they are engine-assigned and immutable) and reports
//! precise failures (LESSONS #4).

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

fn seeded_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
        vec![
            FieldValue::Integer(2),
            FieldValue::Text("Grace".into()),
            FieldValue::Float(2.5),
        ],
    ]
}

fn core_with_rows() -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    core
}

fn rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows("Customers", RowRange { start: 0, len: 100 })
        .expect("read rows")
}

fn failure(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn update(row: usize, values: Vec<FieldValue>) -> Command {
    Command::UpdateRow {
        table: "Customers".into(),
        row,
        values,
    }
}

#[test]
fn update_row_replaces_the_row_and_reports_it() {
    let mut core = core_with_rows();
    let values = vec![
        FieldValue::Integer(1),
        FieldValue::Text("Ada L.".into()),
        FieldValue::Float(1.75),
    ];
    assert_eq!(
        core.dispatch(update(0, values.clone())),
        vec![Event::RowUpdated {
            table: "Customers".into(),
            row: 0
        }]
    );
    let after = rows(&core);
    assert_eq!(after[0], values);
    assert_eq!(after[1], seeded_rows()[1], "other rows are untouched");
}

#[test]
fn a_null_autonumber_keeps_the_current_value() {
    let mut core = core_with_rows();
    let events = core.dispatch(update(
        0,
        vec![
            FieldValue::Null,
            FieldValue::Text("Ada".into()),
            FieldValue::Float(9.0),
        ],
    ));
    assert!(matches!(events[0], Event::RowUpdated { .. }), "{events:?}");
    let after = rows(&core);
    assert_eq!(after[0][0], FieldValue::Integer(1), "id is preserved");
    assert_eq!(after[0][2], FieldValue::Float(9.0));
}

#[test]
fn changing_an_autonumber_is_rejected() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(update(
        0,
        vec![
            FieldValue::Integer(99),
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )));
    assert!(reason.to_lowercase().contains("autonumber"), "{reason}");
    assert!(reason.contains("\"id\""), "{reason}");
    assert_eq!(rows(&core)[0][0], FieldValue::Integer(1));
}

#[test]
fn update_validates_arity_and_kinds() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(update(
        0,
        vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
    )));
    assert!(reason.contains("expected 3 values"), "{reason}");
    assert!(reason.contains("found 2"), "{reason}");

    let reason = failure(&core.dispatch(update(
        0,
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Text("not a number".into()),
        ],
    )));
    assert!(reason.contains("\"score\""), "{reason}");
}

#[test]
fn delete_row_removes_the_row_and_reports_it() {
    let mut core = core_with_rows();
    assert_eq!(
        core.dispatch(Command::DeleteRow {
            table: "customers".into(),
            row: 1,
        }),
        vec![Event::RowDeleted {
            table: "Customers".into(),
            row: 1
        }]
    );
    let after = rows(&core);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0], seeded_rows()[0]);
}

#[test]
fn row_indices_out_of_range_are_rejected() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(update(
        7,
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )));
    assert!(reason.contains("out of range"), "{reason}");
    let reason = failure(&core.dispatch(Command::DeleteRow {
        table: "Customers".into(),
        row: 7,
    }));
    assert!(reason.contains("out of range"), "{reason}");
    assert_eq!(rows(&core).len(), 2);
}

#[test]
fn port_failures_are_reported_and_leave_the_rows_untouched() {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", seeded_rows())
        .with_write_failure("disk is full");
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    let reason = failure(&core.dispatch(update(
        0,
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Float(9.0),
        ],
    )));
    assert!(reason.contains("disk is full"), "{reason}");
    let reason = failure(&core.dispatch(Command::DeleteRow {
        table: "Customers".into(),
        row: 0,
    }));
    assert!(reason.contains("disk is full"), "{reason}");
    assert_eq!(rows(&core), seeded_rows());
}

#[test]
fn row_write_commands_round_trip_through_json() {
    for command in [
        update(
            0,
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("Ada".into()),
                FieldValue::Float(1.5),
            ],
        ),
        Command::DeleteRow {
            table: "Customers".into(),
            row: 1,
        },
    ] {
        let json = serde_json::to_string(&command).expect("serialize command");
        let back: Command = serde_json::from_str(&json).expect("deserialize command");
        assert_eq!(back, command);
    }
}

#[test]
fn update_and_delete_report_missing_database_and_tables() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers()])));
    let values = vec![
        FieldValue::Integer(1),
        FieldValue::Text("Ada".into()),
        FieldValue::Float(1.5),
    ];
    let reason = failure(&core.dispatch(update(0, values.clone())));
    assert!(reason.contains("no database"), "{reason}");
    let reason = failure(&core.dispatch(Command::DeleteRow {
        table: "Customers".into(),
        row: 0,
    }));
    assert!(reason.contains("no database"), "{reason}");

    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    let reason = failure(&core.dispatch(Command::UpdateRow {
        table: "Ghost".into(),
        row: 0,
        values: values.clone(),
    }));
    assert!(reason.contains("not found"), "{reason}");
    let reason = failure(&core.dispatch(Command::DeleteRow {
        table: "Ghost".into(),
        row: 0,
    }));
    assert!(reason.contains("not found"), "{reason}");
}

#[test]
fn a_row_index_at_the_end_is_out_of_range() {
    let mut core = core_with_rows();
    let reason = failure(&core.dispatch(update(
        2,
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Float(1.5),
        ],
    )));
    assert!(reason.contains("out of range"), "{reason}");
}
