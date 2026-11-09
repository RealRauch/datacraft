//! Form-scoped filter contract (phase 64, item M71; ADR-8): the criteria
//! lives with the form, never in the datasheet's table view state. Set
//! validation is syntactic; rows are filtered through the expression
//! service (Access wildcards included).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::FormFilter;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

fn rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![FieldValue::Integer(1), FieldValue::Text("Lena".into())],
        vec![FieldValue::Integer(2), FieldValue::Text("Mika".into())],
        vec![FieldValue::Integer(3), FieldValue::Text("Lars".into())],
    ]
}

fn core() -> Core {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    port.with_row_data("Customers", rows());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

#[test]
fn a_filter_belongs_to_the_form_and_selects_rows() {
    let mut core = core();
    let criteria = "[Name] Like 'L*'";
    let events = core.dispatch(Command::SetFormFilter(FormFilter {
        form: "Card".into(),
        criteria: Some(criteria.into()),
    }));
    assert_eq!(
        events,
        vec![Event::FormFilterSet {
            form: "Card".into()
        }]
    );
    assert_eq!(core.form_filter("Card"), Some(criteria));
    let rows = core
        .read_rows(
            "Customers",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("rows");
    assert_eq!(
        core.form_filter_rows("Card", &rows).expect("filters"),
        vec![0, 2],
        "Access wildcards match Lena and Lars"
    );
}

#[test]
fn clearing_the_filter_restores_every_row() {
    let mut core = core();
    core.dispatch(Command::SetFormFilter(FormFilter {
        form: "Card".into(),
        criteria: Some("[Name] Like 'L*'".into()),
    }));
    let events = core.dispatch(Command::SetFormFilter(FormFilter {
        form: "Card".into(),
        criteria: None,
    }));
    assert_eq!(
        events,
        vec![Event::FormFilterSet {
            form: "Card".into()
        }]
    );
    assert_eq!(core.form_filter("Card"), None);
}

#[test]
fn malformed_criteria_are_rejected_precisely() {
    let mut core = core();
    let events = core.dispatch(Command::SetFormFilter(FormFilter {
        form: "Card".into(),
        criteria: Some("[Name] =".into()),
    }));
    let reason = failed_reason(&events);
    assert!(reason.contains("filter"), "{reason}");
    assert_eq!(core.form_filter("Card"), None);
}

#[test]
fn an_unknown_form_is_rejected() {
    let mut core = core();
    let events = core.dispatch(Command::SetFormFilter(FormFilter {
        form: "Missing".into(),
        criteria: Some("[Name] Like 'L*'".into()),
    }));
    let reason = failed_reason(&events);
    assert!(reason.contains("Missing"), "{reason}");
}
