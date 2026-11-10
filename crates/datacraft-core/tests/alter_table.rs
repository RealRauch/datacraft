//! Contract suite for design-time schema commands (step 24.3, item M9).
//!
//! The core validates against a cloned definition, dispatches exactly one
//! `alter_table` call and refreshes its schema from the backend.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, FieldValue, NumericKind, TableDef,
};

fn core() -> Core {
    let table = TableDef::new(
        "T",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("note", FieldKind::Text { max_len: 20 }),
        ],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![table])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::InsertRow {
        table: "T".to_owned(),
        values: vec![FieldValue::Integer(1), FieldValue::Text("one".to_owned())],
    });
    core
}

fn fields(core: &Core) -> Vec<String> {
    core.inspect().database.expect("open").tables[0]
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect()
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn add_field_appends_and_existing_rows_get_null() {
    let mut core = core();
    let events = core.dispatch(Command::AddField {
        table: "t".to_owned(),
        field: FieldDef::new("extra", FieldKind::Text { max_len: 5 }),
    });
    assert_eq!(
        events,
        vec![Event::TableAltered {
            table: "T".to_owned(),
            field: "extra".to_owned()
        }]
    );
    assert_eq!(fields(&core), ["id", "note", "extra"]);
    let rows = core
        .read_rows("T", datacraft_core::port::RowRange { start: 0, len: 10 })
        .expect("rows");
    assert_eq!(rows[0][2], FieldValue::Null);
}

#[test]
fn rename_and_drop_dispatch_one_change_each() {
    let mut core = core();
    core.dispatch(Command::RenameField {
        table: "T".to_owned(),
        field: "note".to_owned(),
        new_name: "text".to_owned(),
    });
    assert_eq!(fields(&core), ["id", "text"]);
    core.dispatch(Command::DropField {
        table: "T".to_owned(),
        field: "id".to_owned(),
    });
    assert_eq!(fields(&core), ["text"]);
    let rows = core
        .read_rows("T", datacraft_core::port::RowRange { start: 0, len: 10 })
        .expect("rows");
    assert_eq!(rows[0][0], FieldValue::Text("one".to_owned()));
}

#[test]
fn set_field_properties_replaces_the_property_set() {
    let mut core = core();
    let events = core.dispatch(Command::SetFieldProperties {
        table: "T".to_owned(),
        field: "note".to_owned(),
        properties: FieldProperties {
            required: true,
            ..Default::default()
        },
    });
    assert!(
        matches!(events.as_slice(), [Event::TableAltered { .. }]),
        "{events:?}"
    );
    let snapshot = core.inspect().database.expect("open");
    assert!(snapshot.tables[0].fields[1].properties.required);
}

#[test]
fn validation_failures_are_precise_and_leave_state_untouched() {
    let mut core = core();
    let duplicate = failed_reason(&core.dispatch(Command::AddField {
        table: "T".to_owned(),
        field: FieldDef::new("NOTE", FieldKind::Text { max_len: 5 }),
    }));
    assert!(duplicate.contains("duplicate field"), "{duplicate}");
    let unknown_field = failed_reason(&core.dispatch(Command::DropField {
        table: "T".to_owned(),
        field: "missing".to_owned(),
    }));
    assert!(unknown_field.contains("not found"), "{unknown_field}");
    let unknown_table = failed_reason(&core.dispatch(Command::RenameField {
        table: "missing".to_owned(),
        field: "id".to_owned(),
        new_name: "x".to_owned(),
    }));
    assert!(unknown_table.contains("not found"), "{unknown_table}");
    assert_eq!(fields(&core), ["id", "note"]);
}

/// The alter gate answers precisely when no database is open and when
/// the driver lacks the capability (phase 30, M17).
#[test]
fn alter_table_gates_are_precise() {
    let mut closed = Core::new(Box::new(MockPort::with_tables(vec![TableDef::new(
        "T",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )])));
    let events = closed.dispatch(Command::AddField {
        table: "T".to_owned(),
        field: FieldDef::new("extra", FieldKind::Text { max_len: 5 }),
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("closed core refuses");
    assert!(reason.contains("no database is open"), "{reason}");

    let mut caps = datacraft_core::port::DriverCapabilities::full();
    caps.alter_table = false;
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "T",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    port.with_capabilities(caps);
    let mut restricted = Core::new(Box::new(port));
    restricted.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = restricted.dispatch(Command::AddField {
        table: "T".to_owned(),
        field: FieldDef::new("extra", FieldKind::Text { max_len: 5 }),
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("restricted core refuses");
    assert!(reason.contains("cannot alter the schema"), "{reason}");
}

/// DeleteRow on an out-of-range index answers precisely instead of
/// panicking or silently succeeding.
#[test]
fn delete_row_out_of_range_is_reported() {
    let table = TableDef::new(
        "T",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![table])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let events = core.dispatch(Command::DeleteRow {
        table: "T".to_owned(),
        row: 9,
    });
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("out of range refuses");
    assert!(reason.contains("out of range"), "{reason}");
}
