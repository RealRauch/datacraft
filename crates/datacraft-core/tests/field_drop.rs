//! Drag-and-drop field binding contract (phase 102, step 102.4; item M133).
//!
//! Dropping a record-source field on the canvas places a bound control at
//! the drop point: snapped to the grid, clamped inside the canvas/page,
//! named uniquely and validated against the source whitelist. The click
//! path (`AddFormField`/`AddReportField`) keeps its next-free-slot rule.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::form::{ControlKind, DesignCommand};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("photo", FieldKind::Binary(0)),
        ],
    )
}

fn opened_with_form() -> Core {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
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

fn drop_field(core: &mut Core, object: ObjectId, field: &str, at: (f32, f32)) -> Vec<Event> {
    core.dispatch(Command::Design(DesignCommand::PlaceFieldControl {
        object,
        field: field.to_owned(),
        at,
    }))
}

fn form_id() -> ObjectId {
    ObjectId::new(ObjectKind::Form, "Card")
}

fn failed(events: &[Event]) -> (FailureCode, String) {
    match events {
        [Event::Failed { code, reason }] => (*code, reason.clone()),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn a_dropped_field_lands_snapped_at_the_drop_point() {
    let mut core = opened_with_form();
    let events = drop_field(&mut core, form_id(), "name", (100.0, 50.0));
    assert_eq!(
        events,
        vec![Event::ControlAdded {
            form: "Card".into(),
            control: "name".into(),
        }]
    );
    let control = core.inspect().database.unwrap().forms[0].controls[0].clone();
    assert_eq!(
        control.rect,
        Rect {
            x: 104.0,
            y: 48.0,
            width: 120.0,
            height: 24.0,
        },
        "the drop point snaps to the 8 pt grid"
    );
    assert_eq!(
        control.kind,
        ControlKind::TextBox {
            bound_field: Some("name".into())
        }
    );
    assert_eq!(control.properties.tab_index, 0, "it lands last");
}

#[test]
fn a_boolean_field_drops_as_a_checkbox() {
    let mut core = opened_with_form();
    drop_field(&mut core, form_id(), "active", (8.0, 8.0));
    let control = core.inspect().database.unwrap().forms[0].controls[0].clone();
    assert!(matches!(
        control.kind,
        ControlKind::CheckBox {
            bound_field: Some(_)
        }
    ));
}

#[test]
fn a_drop_near_the_edge_stays_inside_the_canvas() {
    let mut core = opened_with_form();
    drop_field(&mut core, form_id(), "name", (636.0, 396.0));
    let rect = core.inspect().database.unwrap().forms[0].controls[0].rect;
    assert_eq!(rect.x, 520.0, "{rect:?}");
    assert_eq!(rect.y, 376.0, "{rect:?}");
    assert!(rect.x + rect.width <= 640.0);
    assert!(rect.y + rect.height <= 400.0);
}

#[test]
fn repeated_drops_name_the_controls_uniquely() {
    let mut core = opened_with_form();
    drop_field(&mut core, form_id(), "name", (8.0, 8.0));
    drop_field(&mut core, form_id(), "name", (8.0, 48.0));
    let names: Vec<String> = core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|control| control.name.clone())
        .collect();
    assert_eq!(names, ["name", "name2"]);
    let tab: Vec<u32> = core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|control| control.properties.tab_index)
        .collect();
    assert_eq!(tab, [0, 1], "each drop lands last in the tab order");
}

#[test]
fn a_report_drops_on_its_page() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Customers".into(),
    });
    let events = drop_field(
        &mut core,
        ObjectId::new(ObjectKind::Report, "Weekly"),
        "name",
        (300.0, 500.0),
    );
    assert_eq!(
        events,
        vec![Event::ReportControlAdded {
            report: "Weekly".into(),
            control: "name".into(),
        }]
    );
    let rect = core.inspect().database.unwrap().reports[0].controls[0].rect;
    assert_eq!(
        rect,
        Rect {
            x: 296.0,
            y: 504.0,
            width: 120.0,
            height: 24.0,
        }
    );
}

#[test]
fn drop_errors_are_precise_and_leave_the_design_unchanged() {
    // No record source.
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    let (code, reason) = failed(drop_field(&mut core, form_id(), "name", (8.0, 8.0)));
    assert_eq!(code, FailureCode::InvalidInput);
    assert!(reason.to_lowercase().contains("record source"), "{reason}");

    // A source without the field.
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    let (code, reason) = failed(drop_field(&mut core, form_id(), "ghost", (8.0, 8.0)));
    assert_eq!(code, FailureCode::InvalidInput);
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");

    // A field with no designer control (binary).
    let (code, reason) = failed(drop_field(&mut core, form_id(), "photo", (8.0, 8.0)));
    assert_eq!(code, FailureCode::InvalidInput);
    assert!(reason.to_lowercase().contains("photo"), "{reason}");

    // A non-design object kind.
    let (code, reason) = failed(drop_field(
        &mut core,
        ObjectId::new(ObjectKind::Table, "Customers"),
        "name",
        (8.0, 8.0),
    ));
    assert_eq!(code, FailureCode::InvalidInput);
    assert!(
        reason.to_lowercase().contains("form") && reason.to_lowercase().contains("report"),
        "{reason}"
    );

    assert!(
        core.inspect().database.unwrap().forms[0].controls.is_empty(),
        "no refused drop changed the design"
    );
}
