//! Report command guards (phase 24 coverage gate).
//!
//! Every guard arm — no database, unknown report, grouping without a source,
//! unknown source table/field — answers precisely instead of silently
//! assuming state.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected Failed, got {other:?}"),
    }
}

fn core() -> Core {
    let table = TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
        ],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![table])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Rpt".to_owned(),
    });
    core
}

#[test]
fn report_commands_without_a_database_fail_precisely() {
    let mut core = Core::new(Box::new(MockPort::new()));
    for command in [
        Command::AddReportControl {
            report: "Rpt".to_owned(),
            control: datacraft_core::form::ControlDef {
                name: "c1".to_owned(),
                kind: datacraft_core::form::ControlKind::Label {
                    text: "label".to_owned(),
                },
                rect: datacraft_core::canvas::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
            },
        },
        Command::SetReportControlRect {
            report: "Rpt".to_owned(),
            control: "c1".to_owned(),
            rect: datacraft_core::canvas::Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
        },
        Command::DeleteReportControl {
            report: "Rpt".to_owned(),
            control: "c1".to_owned(),
        },
        Command::AddReportField {
            report: "Rpt".to_owned(),
            field: "id".to_owned(),
        },
        Command::SetReportSource {
            report: "Rpt".to_owned(),
            table: "Customers".to_owned(),
        },
    ] {
        let events = core.dispatch(command);
        assert!(reason(&events).contains("no database"), "{events:?}");
    }
}

#[test]
fn unknown_reports_and_sources_are_rejected() {
    let mut core = core();
    for command in [
        Command::DeleteReport {
            name: "Missing".to_owned(),
        },
        Command::AddReportField {
            report: "Missing".to_owned(),
            field: "id".to_owned(),
        },
        Command::SetReportGrouping {
            report: "Missing".to_owned(),
            group_by: None,
        },
    ] {
        let events = core.dispatch(command);
        assert!(reason(&events).contains("not found"), "{events:?}");
    }

    let events = core.dispatch(Command::SetReportSource {
        report: "Rpt".to_owned(),
        table: "NoSuchTable".to_owned(),
    });
    assert!(reason(&events).contains("not found"), "{events:?}");
}

#[test]
fn grouping_requires_a_source_and_a_known_field() {
    let mut core = core();
    let events = core.dispatch(Command::SetReportGrouping {
        report: "Rpt".to_owned(),
        group_by: Some("name".to_owned()),
    });
    assert!(
        reason(&events).contains("source table"),
        "grouping before a source: {events:?}"
    );

    core.dispatch(Command::SetReportSource {
        report: "Rpt".to_owned(),
        table: "Customers".to_owned(),
    });
    let events = core.dispatch(Command::SetReportGrouping {
        report: "Rpt".to_owned(),
        group_by: Some("missing".to_owned()),
    });
    assert!(reason(&events).contains("not found"), "{events:?}");
    core.dispatch(Command::SetReportGrouping {
        report: "Rpt".to_owned(),
        group_by: Some("name".to_owned()),
    });
    assert!(core.report_preview("Rpt").is_ok());
}
