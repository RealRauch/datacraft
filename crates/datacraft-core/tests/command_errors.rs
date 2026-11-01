//! Precise failure answers for the remaining command branches.
//!
//! Every rejected command names its rule; these tests pin the diagnostic
//! arms that the happy-path suites do not reach (LESSONS #4).

use datacraft_core::ObjectId;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::{QbeRow, QbeValue};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn core() -> Core {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "T",
        vec![FieldDef::new("n", FieldKind::Memo)],
    )]);
    port.with_row_data("T", vec![]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    core
}

fn failure(events: &[Event]) -> String {
    match events.first() {
        Some(Event::Failed { reason }) => reason.clone(),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn forms_open_like_every_other_object() {
    let mut core = core();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Card",
    )));
    assert!(
        matches!(&events[0], Event::TabOpened { object, .. } if object.name == "Card"),
        "{events:?}"
    );
}

#[test]
fn query_names_must_not_be_blank() {
    let mut core = core();
    let events = core.dispatch(Command::CreateQuery { name: "   ".into() });
    assert!(failure(&events).contains("invalid name"), "{events:?}");
}

#[test]
fn qbe_edits_report_the_rule_they_violate() {
    let mut core = core();
    let no_database = failure(&Core::new(Box::new(MockPort::new())).dispatch(
        Command::SetQbeCell {
            query: "Q".into(),
            row: QbeRow::Table,
            col: 0,
            value: QbeValue::Text("T".into()),
        },
    ));
    assert!(no_database.contains("no database"), "{no_database}");

    let unknown = failure(&core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("T".into()),
    }));
    assert!(unknown.contains("not found"), "{unknown}");

    core.dispatch(Command::CreateQuery { name: "Q".into() });
    let table_bool = failure(&core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Bool(true),
    }));
    assert!(table_bool.contains("takes a name"), "{table_bool}");

    let no_source = failure(&core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("n".into()),
    }));
    assert!(no_source.contains("source table"), "{no_source}");

    core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("T".into()),
    });
    let field_bool = failure(&core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Bool(false),
    }));
    assert!(field_bool.contains("takes a name"), "{field_bool}");
}

#[test]
fn closed_tabs_must_exist() {
    let mut core = core();
    let events = core.dispatch(Command::CloseTab {
        tab_id: datacraft_core::TabId::new(99),
    });
    assert!(failure(&events).contains("unknown tab"), "{events:?}");
}

#[test]
fn queries_without_their_source_table_are_reported() {
    let mut core = core();
    core.dispatch(Command::CreateQuery { name: "Q".into() });
    core.dispatch(Command::SetQbeCell {
        query: "Q".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("T".into()),
    });
    core.dispatch(Command::DropTable { table: "T".into() });
    let events = core.dispatch(Command::RunQuery { query: "Q".into() });
    assert!(failure(&events).contains("source table"), "{events:?}");
}
