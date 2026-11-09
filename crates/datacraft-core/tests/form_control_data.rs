//! Combo row-source contract (phase 64, M70; ADR-8): a table or saved
//! query plus a display field, validated before the mutation. A table
//! source canonicalizes its names; an empty source clears the list.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlData, ControlDef, ControlKind, SetControlData};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Email", FieldKind::Text { max_len: 80 }),
        ],
    )
}

fn combo(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::ComboBox {
            bound_field: Some("Name".into()),
            row_source: String::new(),
            display_field: String::new(),
        },
        rect: Rect {
            x: 8.0,
            y: 8.0,
            width: 160.0,
            height: 24.0,
        },
    }
}

fn label(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label {
            text: "Pick one".into(),
        },
        rect: Rect {
            x: 8.0,
            y: 40.0,
            width: 80.0,
            height: 20.0,
        },
    }
}

fn opened_core() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateTable {
        definition: customers(),
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: combo("Pick"),
    });
    core
}

fn set_source(core: &mut Core, row_source: &str, display_field: &str) -> Vec<Event> {
    core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "Pick".into(),
        data: ControlData::ComboSource {
            row_source: row_source.into(),
            display_field: display_field.into(),
        },
    }))
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn combo_kind(core: &Core) -> ControlKind {
    core.inspect()
        .database
        .as_ref()
        .expect("database")
        .forms
        .iter()
        .find(|form| form.name == "Card")
        .and_then(|form| form.controls.iter().find(|control| control.name == "Pick"))
        .map(|control| control.kind.clone())
        .expect("combo control")
}

#[test]
fn a_table_source_canonicalizes_the_names() {
    let mut core = opened_core();
    let events = set_source(&mut core, "customers", "email");
    assert_eq!(
        events,
        vec![Event::ControlDataSet {
            form: "Card".into(),
            control: "Pick".into(),
        }]
    );
    match combo_kind(&core) {
        ControlKind::ComboBox {
            row_source,
            display_field,
            ..
        } => {
            assert_eq!(row_source, "Customers", "the table name is canonical");
            assert_eq!(display_field, "Email", "the field name is canonical");
        }
        other => panic!("expected a combo box, got {other:?}"),
    }
}

#[test]
fn an_unknown_table_is_rejected() {
    let mut core = opened_core();
    let reason = failed_reason(&set_source(&mut core, "Missing", "Email"));
    assert!(reason.contains("Missing"), "{reason}");
}

#[test]
fn an_unknown_display_field_is_rejected() {
    let mut core = opened_core();
    let reason = failed_reason(&set_source(&mut core, "Customers", "Phone"));
    assert!(reason.contains("Phone"), "{reason}");
}

#[test]
fn a_non_combo_control_is_rejected() {
    let mut core = opened_core();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("Title"),
    });
    let events = core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "Title".into(),
        data: ControlData::ComboSource {
            row_source: "Customers".into(),
            display_field: "Email".into(),
        },
    }));
    let reason = failed_reason(&events);
    assert!(reason.contains("combo") || reason.contains("Combo"), "{reason}");
}

#[test]
fn an_empty_source_clears_the_list() {
    let mut core = opened_core();
    set_source(&mut core, "Customers", "Email");
    let events = set_source(&mut core, "", "");
    assert!(matches!(events.as_slice(), [Event::ControlDataSet { .. }]));
    match combo_kind(&core) {
        ControlKind::ComboBox {
            row_source,
            display_field,
            ..
        } => {
            assert!(row_source.is_empty());
            assert!(display_field.is_empty());
        }
        other => panic!("expected a combo box, got {other:?}"),
    }
}

#[test]
fn an_unknown_query_source_is_rejected() {
    let mut core = opened_core();
    let reason = failed_reason(&set_source(&mut core, "MissingQuery", "Email"));
    assert!(reason.contains("MissingQuery"), "{reason}");
}
