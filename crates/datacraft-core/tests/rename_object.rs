//! Contract suite for object renames (phase 72, step 72.1, item M81).
//!
//! One kind-tagged `RenameObject` command renames tables, queries, forms
//! and reports. Tables rename through the port; sidecar objects rename in
//! place. Open tabs, datasheet view state, form filters and every
//! structural sidecar reference follow the new name; stored view SQL is
//! free text and deliberately not rewritten.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ButtonAction, ControlData, ControlDef, ControlKind, SetControlData};
use datacraft_core::port::{DriverCapabilities, MockPort, Relationship};
use datacraft_core::qbe::{QbeRow, QbeValue, SortOrder};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::view::ViewCommand;
use datacraft_core::{ObjectId, ObjectKind};

fn table(name: &str) -> TableDef {
    TableDef::new(
        name,
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn opened() -> Core {
    let mut port = MockPort::with_tables(vec![table("Customers"), table("Orders")]);
    port.with_relationships(vec![Relationship {
        table: "Orders".into(),
        column: "customer_id".into(),
        related_table: "Customers".into(),
        related_column: "id".into(),
    }]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core
}

fn rename(kind: ObjectKind, name: &str, new_name: &str) -> Command {
    Command::RenameObject {
        object: ObjectId::new(kind, name),
        new_name: new_name.to_owned(),
    }
}

fn renamed(events: &[Event]) -> (&ObjectKind, &str, &str) {
    match events {
        [
            Event::ObjectRenamed {
                kind,
                old_name,
                new_name,
            },
        ] => (kind, old_name, new_name),
        other => panic!("expected one ObjectRenamed event, got {other:?}"),
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

fn control(name: &str, kind: ControlKind) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind,
        rect: Rect {
            x: 8.0,
            y: 8.0,
            width: 120.0,
            height: 24.0,
        },
    }
}

#[test]
fn table_rename_moves_tabs_view_state_and_relationships() {
    let mut core = opened();
    core.dispatch(Command::InsertRow {
        table: "Customers".into(),
        values: vec![
            datacraft_core::schema::FieldValue::Integer(1),
            datacraft_core::schema::FieldValue::Text("Ada".into()),
        ],
    });
    core.dispatch(Command::SetTableSort {
        table: "Customers".into(),
        field: "name".into(),
        order: SortOrder::Ascending,
    });
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));

    let events = core.dispatch(rename(ObjectKind::Table, "Customers", "Clients"));
    let (kind, old, new) = renamed(&events);
    assert_eq!(*kind, ObjectKind::Table);
    assert_eq!(old, "Customers");
    assert_eq!(new, "Clients");

    let snapshot = core.inspect();
    let database = snapshot.database.as_ref().expect("open");
    assert_eq!(database.tables[0].name, "Clients");
    assert_eq!(database.relationships[0].related_table, "Clients");
    let tab = snapshot.tabs.first().expect("tab");
    assert_eq!(tab.object_name, "Clients");
    assert_eq!(tab.sort_field.as_deref(), Some("name"));
    // The row survived the rename and the old name is gone.
    let rows = core
        .read_rows(
            "Clients",
            datacraft_core::port::RowRange { start: 0, len: 10 },
        )
        .expect("rows under the new name");
    assert_eq!(
        rows[0][1],
        datacraft_core::schema::FieldValue::Text("Ada".into())
    );
    assert!(
        core.read_rows(
            "Customers",
            datacraft_core::port::RowRange { start: 0, len: 10 }
        )
        .is_err()
    );
}

#[test]
fn table_rename_follows_sidecar_references() {
    let mut core = opened();
    core.dispatch(Command::CreateQuery { name: "Q1".into() });
    core.dispatch(Command::SetQbeCell {
        query: "Q1".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: control(
            "picker",
            ControlKind::ComboBox {
                bound_field: None,
                row_source: "Customers".into(),
                display_field: "name".into(),
            },
        ),
    });
    core.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Sheet".into(),
        table: "Customers".into(),
    });

    core.dispatch(rename(ObjectKind::Table, "Customers", "Clients"));

    let database = core.inspect().database.expect("open");
    assert_eq!(database.queries[0].table, "Clients");
    assert_eq!(database.forms[0].source, "Clients");
    assert_eq!(database.reports[0].source, "Clients");
    assert_eq!(
        database.forms[0].controls[0].kind,
        ControlKind::ComboBox {
            bound_field: None,
            row_source: "Clients".into(),
            display_field: "name".into(),
        }
    );
}

#[test]
fn query_rename_follows_tabs_combo_sources_and_button_actions() {
    let mut core = opened();
    core.dispatch(Command::CreateQuery { name: "Q1".into() });
    core.dispatch(Command::SetQbeCell {
        query: "Q1".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: control(
            "picker",
            ControlKind::ComboBox {
                bound_field: None,
                row_source: "Q1".into(),
                display_field: "name".into(),
            },
        ),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: control(
            "run",
            ControlKind::Button {
                caption: "Run".into(),
                action: None,
            },
        ),
    });
    core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "run".into(),
        data: ControlData::Button {
            action: Some(ButtonAction::RunQuery { query: "Q1".into() }),
        },
    }));
    core.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::Query, "Q1")));

    let events = core.dispatch(rename(ObjectKind::Query, "Q1", "Q2"));
    let (kind, old, new) = renamed(&events);
    assert_eq!(*kind, ObjectKind::Query);
    assert_eq!((old, new), ("Q1", "Q2"));

    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs[0].object_name, "Q2");
    let form = &snapshot.database.expect("open").forms[0];
    assert!(matches!(
        &form.controls[0].kind,
        ControlKind::ComboBox { row_source, .. } if row_source == "Q2"
    ));
    assert!(matches!(
        &form.controls[1].kind,
        ControlKind::Button {
            action: Some(ButtonAction::RunQuery { query }),
            ..
        } if query == "Q2"
    ));
}

#[test]
fn form_rename_follows_tabs_subforms_filters_and_button_actions() {
    let mut core = opened();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::CreateForm {
        name: "Host".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Host".into(),
        control: control(
            "embedded",
            ControlKind::SubForm {
                form: "Card".into(),
            },
        ),
    });
    core.dispatch(Command::AddFormControl {
        form: "Host".into(),
        control: control(
            "open",
            ControlKind::Button {
                caption: "Open".into(),
                action: None,
            },
        ),
    });
    core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Host".into(),
        control: "open".into(),
        data: ControlData::Button {
            action: Some(ButtonAction::OpenForm {
                form: "Card".into(),
            }),
        },
    }));
    core.dispatch(Command::SetFormFilter(datacraft_core::form::FormFilter {
        form: "Card".into(),
        criteria: Some("id = 1".into()),
    }));
    core.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::Form, "Card")));

    let events = core.dispatch(rename(ObjectKind::Form, "Card", "ClientCard"));
    let (kind, old, new) = renamed(&events);
    assert_eq!(*kind, ObjectKind::Form);
    assert_eq!((old, new), ("Card", "ClientCard"));

    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs[0].object_name, "ClientCard");
    let host = snapshot
        .database
        .as_ref()
        .expect("open")
        .forms
        .iter()
        .find(|form| form.name == "Host")
        .expect("host form");
    assert!(matches!(
        &host.controls[0].kind,
        ControlKind::SubForm { form } if form == "ClientCard"
    ));
    assert!(matches!(
        &host.controls[1].kind,
        ControlKind::Button {
            action: Some(ButtonAction::OpenForm { form }),
            ..
        } if form == "ClientCard"
    ));
    assert_eq!(core.form_filter("ClientCard"), Some("id = 1"));
    assert!(!core.state().form_filters.contains_key("Card"));
}

#[test]
fn report_rename_follows_tabs_and_button_actions() {
    let mut core = opened();
    core.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: control(
            "print",
            ControlKind::Button {
                caption: "Print".into(),
                action: None,
            },
        ),
    });
    core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "print".into(),
        data: ControlData::Button {
            action: Some(ButtonAction::OpenReport {
                report: "Sheet".into(),
            }),
        },
    }));
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Report,
        "Sheet",
    )));

    let events = core.dispatch(rename(ObjectKind::Report, "Sheet", "Invoice"));
    let (kind, old, new) = renamed(&events);
    assert_eq!(*kind, ObjectKind::Report);
    assert_eq!((old, new), ("Sheet", "Invoice"));

    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs[0].object_name, "Invoice");
    let form = &snapshot.database.expect("open").forms[0];
    assert!(matches!(
        &form.controls[0].kind,
        ControlKind::Button {
            action: Some(ButtonAction::OpenReport { report }),
            ..
        } if report == "Invoice"
    ));
}

#[test]
fn names_are_validated_and_collisions_are_precise() {
    let mut core = opened();
    let empty = failed_reason(&core.dispatch(rename(ObjectKind::Table, "Customers", "   ")));
    assert!(empty.contains("empty"), "{empty}");
    let duplicate = failed_reason(&core.dispatch(rename(ObjectKind::Table, "Customers", "orders")));
    assert!(duplicate.contains("duplicate"), "{duplicate}");
    let unknown = failed_reason(&core.dispatch(rename(ObjectKind::Table, "Ghost", "X")));
    assert!(unknown.contains("not found"), "{unknown}");
    let view_kind = failed_reason(&core.dispatch(rename(ObjectKind::Macro, "M", "N")));
    assert!(view_kind.contains("cannot be renamed"), "{view_kind}");

    // Renaming a table onto a sidecar view's name would break the shared
    // table/view namespace — it is refused.
    core.dispatch(Command::View(ViewCommand::Create {
        name: "Listing".into(),
        sql: "SELECT * FROM Customers".into(),
    }));
    let clash = failed_reason(&core.dispatch(rename(ObjectKind::Table, "Customers", "listing")));
    assert!(clash.contains("duplicate"), "{clash}");

    assert_eq!(
        core.state().database.as_ref().expect("open").tables[0].name,
        "Customers"
    );
}

#[test]
fn case_only_renames_are_allowed() {
    let mut core = opened();
    let events = core.dispatch(rename(ObjectKind::Table, "customers", "CUSTOMERS"));
    let (_, old, new) = renamed(&events);
    assert_eq!(old, "Customers");
    assert_eq!(new, "CUSTOMERS");
    assert_eq!(
        core.state().database.as_ref().expect("open").tables[0].name,
        "CUSTOMERS"
    );
}

#[test]
fn rename_gates_are_precise() {
    let mut closed = Core::new(Box::new(MockPort::new()));
    let reason = failed_reason(&closed.dispatch(rename(ObjectKind::Table, "T", "U")));
    assert!(reason.contains("no database is open"), "{reason}");

    let mut caps = DriverCapabilities::full();
    caps.rename_table = false;
    let mut port = MockPort::with_tables(vec![table("T")]);
    port.with_capabilities(caps);
    let mut restricted = Core::new(Box::new(port));
    restricted.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    let reason = failed_reason(&restricted.dispatch(rename(ObjectKind::Table, "T", "U")));
    assert!(reason.contains("cannot rename tables"), "{reason}");
}

#[test]
fn table_renames_clear_the_row_history_sidecar_renames_keep_it() {
    let mut core = opened();
    core.dispatch(Command::InsertRow {
        table: "Customers".into(),
        values: vec![
            datacraft_core::schema::FieldValue::Integer(1),
            datacraft_core::schema::FieldValue::Text("Ada".into()),
        ],
    });
    assert_eq!(core.state().database.as_ref().expect("open").undo.len(), 1);
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(rename(ObjectKind::Form, "Card", "ClientCard"));
    assert_eq!(core.state().database.as_ref().expect("open").undo.len(), 1);
    core.dispatch(rename(ObjectKind::Table, "Customers", "Clients"));
    assert_eq!(core.state().database.as_ref().expect("open").undo.len(), 0);
}

#[test]
fn rename_command_round_trips_on_the_wire() {
    let command = rename(ObjectKind::Query, "Q1", "Q2");
    let json = serde_json::to_string(&command).expect("serialize");
    let back: Command = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, command);
}
