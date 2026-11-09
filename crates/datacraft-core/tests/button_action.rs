//! Command-button action contract (phase 64, item M69; ADR-8): the action
//! vocabulary is open form / open report / run query / apply form filter.
//! Targets are validated and canonicalized before the mutation; the empty
//! action stays a valid state.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ButtonAction, ControlData, ControlDef, ControlKind, SetControlData};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn button(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Button {
            caption: "Go".into(),
            action: None,
        },
        rect: Rect {
            x: 8.0,
            y: 8.0,
            width: 120.0,
            height: 24.0,
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
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: button("Go"),
    });
    core.dispatch(Command::CreateForm {
        name: "Target".into(),
    });
    core.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    core.dispatch(Command::CreateQuery {
        name: "Pick".into(),
    });
    core
}

fn set_action(core: &mut Core, action: Option<ButtonAction>) -> Vec<Event> {
    core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "Go".into(),
        data: ControlData::Button { action },
    }))
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn action(core: &Core) -> Option<ButtonAction> {
    let db = core.inspect().database.expect("database");
    let form = db
        .forms
        .iter()
        .find(|form| form.name == "Card")
        .expect("form");
    let control = form
        .controls
        .iter()
        .find(|control| control.name == "Go")
        .expect("control");
    match &control.kind {
        ControlKind::Button { action, .. } => action.clone(),
        other => panic!("expected a button, got {other:?}"),
    }
}

#[test]
fn an_open_form_action_canonicalizes_the_target() {
    let mut core = opened_core();
    let events = set_action(
        &mut core,
        Some(ButtonAction::OpenForm {
            form: "target".into(),
        }),
    );
    assert_eq!(
        events,
        vec![Event::ControlDataSet {
            form: "Card".into(),
            control: "Go".into(),
        }]
    );
    assert_eq!(
        action(&core),
        Some(ButtonAction::OpenForm {
            form: "Target".into()
        })
    );
}

#[test]
fn an_open_report_action_canonicalizes_the_target() {
    let mut core = opened_core();
    set_action(
        &mut core,
        Some(ButtonAction::OpenReport {
            report: "sheet".into(),
        }),
    );
    assert_eq!(
        action(&core),
        Some(ButtonAction::OpenReport {
            report: "Sheet".into()
        })
    );
}

#[test]
fn a_run_query_action_canonicalizes_the_target() {
    let mut core = opened_core();
    set_action(
        &mut core,
        Some(ButtonAction::RunQuery {
            query: "pick".into(),
        }),
    );
    assert_eq!(
        action(&core),
        Some(ButtonAction::RunQuery {
            query: "Pick".into()
        })
    );
}

#[test]
fn an_apply_filter_action_keeps_the_predicate() {
    let mut core = opened_core();
    let criteria = " [Name] Like 'L*' ";
    set_action(
        &mut core,
        Some(ButtonAction::ApplyFilter {
            criteria: criteria.into(),
        }),
    );
    assert_eq!(
        action(&core),
        Some(ButtonAction::ApplyFilter {
            criteria: "[Name] Like 'L*'".into()
        })
    );
}

#[test]
fn a_malformed_filter_predicate_is_rejected() {
    let mut core = opened_core();
    let reason = failed_reason(&set_action(
        &mut core,
        Some(ButtonAction::ApplyFilter {
            criteria: "[Name] =".into(),
        }),
    ));
    assert!(
        reason.contains("filter") || reason.contains("criteria"),
        "{reason}"
    );
    assert_eq!(action(&core), None);
}

#[test]
fn unknown_targets_are_rejected_precisely() {
    let mut core = opened_core();
    for candidate in [
        ButtonAction::OpenForm {
            form: "Missing".into(),
        },
        ButtonAction::OpenReport {
            report: "Missing".into(),
        },
        ButtonAction::RunQuery {
            query: "Missing".into(),
        },
    ] {
        let reason = failed_reason(&set_action(&mut core, Some(candidate.clone())));
        assert!(reason.contains("Missing"), "{candidate:?}: {reason}");
        assert_eq!(action(&core), None, "{candidate:?} left no action");
    }
}

#[test]
fn a_non_button_control_is_rejected() {
    let mut core = opened_core();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: ControlDef {
            name: "Label".into(),
            kind: ControlKind::Label { text: "x".into() },
            rect: Rect {
                x: 8.0,
                y: 40.0,
                width: 80.0,
                height: 20.0,
            },
        },
    });
    let events = core.dispatch(Command::SetFormControlData(SetControlData {
        form: "Card".into(),
        control: "Label".into(),
        data: ControlData::Button {
            action: Some(ButtonAction::OpenForm {
                form: "Target".into(),
            }),
        },
    }));
    let reason = failed_reason(&events);
    assert!(reason.contains("button"), "{reason}");
}

#[test]
fn an_empty_action_clears() {
    let mut core = opened_core();
    set_action(
        &mut core,
        Some(ButtonAction::OpenForm {
            form: "Target".into(),
        }),
    );
    let events = set_action(&mut core, None);
    assert!(matches!(events.as_slice(), [Event::ControlDataSet { .. }]));
    assert_eq!(action(&core), None);
}
