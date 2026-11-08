//! InsertRow command contract (step 13.2, item H36).
//!
//! The bus validates the row against the table schema, writes through the
//! port first and only then reports the event. Validation failures are
//! precise and name the rule (LESSONS #4).

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn customers() -> TableDef {
    let mut name = FieldDef::new("name", FieldKind::Text { max_len: 20 });
    name.properties.required = true;
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            name,
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

fn open_core(port: MockPort) -> Core {
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    assert!(
        matches!(events[0], Event::DatabaseOpened { .. }),
        "{events:?}"
    );
    core
}

fn core_with_table() -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", Vec::new());
    open_core(port)
}

fn row(name: &str) -> Vec<FieldValue> {
    vec![
        FieldValue::Null,
        FieldValue::Text(name.into()),
        FieldValue::Boolean(true),
        FieldValue::Float(1.5),
    ]
}

fn insert(table: &str, values: Vec<FieldValue>) -> Command {
    Command::InsertRow {
        table: table.into(),
        values,
    }
}

fn failure(events: &[Event]) -> &str {
    match events.first() {
        Some(Event::Failed { reason, .. }) => reason,
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

#[test]
fn insert_row_appends_through_the_port_and_reports_the_canonical_table() {
    let mut core = core_with_table();
    let events = core.dispatch(insert("customers", row("Ada")));
    assert_eq!(
        events,
        vec![Event::RowInserted {
            table: "Customers".into()
        }]
    );
    let rows = core
        .read_rows("Customers", RowRange { start: 0, len: 10 })
        .expect("read back the inserted row");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
}

#[test]
fn a_null_autonumber_means_engine_assigned() {
    let mut core = core_with_table();
    let events = core.dispatch(insert("Customers", row("Ada")));
    assert!(matches!(events[0], Event::RowInserted { .. }), "{events:?}");

    let mut user_value = row("Grace");
    user_value[0] = FieldValue::Integer(99);
    let events = core.dispatch(insert("Customers", user_value));
    assert!(failure(&events).contains("engine-assigned"), "{events:?}");
}

#[test]
fn arity_mismatch_names_expected_and_found_counts() {
    let mut core = core_with_table();
    let events = core.dispatch(insert("Customers", vec![FieldValue::Null]));
    let reason = failure(&events);
    assert!(reason.contains("expected 4 values"), "{reason}");
    assert!(reason.contains("found 1"), "{reason}");
}

#[test]
fn unknown_tables_are_rejected() {
    let mut core = core_with_table();
    let events = core.dispatch(insert("Missing", row("Ada")));
    assert!(failure(&events).contains("not found"), "{events:?}");
}

#[test]
fn kind_mismatches_name_the_field_and_both_families() {
    let mut core = core_with_table();
    let mut values = row("Ada");
    values[1] = FieldValue::Integer(7);
    let reason = failure(&core.dispatch(insert("Customers", values))).to_owned();
    assert!(reason.contains("\"name\""), "{reason}");
    assert!(reason.contains("short text"), "{reason}");
    assert!(reason.contains("integer"), "{reason}");
}

#[test]
fn required_fields_reject_null() {
    let mut core = core_with_table();
    let mut values = row("Ada");
    values[1] = FieldValue::Null;
    let reason = failure(&core.dispatch(insert("Customers", values))).to_owned();
    assert!(reason.contains("required"), "{reason}");
}

#[test]
fn port_failures_are_reported_and_leave_the_rows_untouched() {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", Vec::new())
        .with_write_failure("disk is full");
    let mut core = open_core(port);
    let events = core.dispatch(insert("Customers", row("Ada")));
    assert!(failure(&events).contains("disk is full"), "{events:?}");
    let rows = core
        .read_rows("Customers", RowRange { start: 0, len: 10 })
        .expect("rows stay readable");
    assert!(
        rows.is_empty(),
        "the rejected row must not appear: {rows:?}"
    );
}

#[test]
fn inserting_without_an_open_database_fails() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers()])));
    let events = core.dispatch(insert("Customers", row("Ada")));
    assert!(failure(&events).contains("no database"), "{events:?}");
}

#[test]
fn insert_row_commands_round_trip_through_json() {
    let command = insert("Customers", row("Ada"));
    let json = serde_json::to_string(&command).expect("serialize command");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(back, command);
}
