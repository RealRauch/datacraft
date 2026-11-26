//! Contract suite for the designer order surfaces (phase 102, step 102.3;
//! item M133): z-order (paint order, back to front) and tab order (focus
//! order) are independent, both move exactly one control per command and
//! both clamp out-of-range positions. The paint order is the control
//! vector order (what the canvas and the print preview walk); the tab
//! order is the persisted `tab_index` property, renumbered densely.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{
    ControlDef, ControlKind, ControlOrder, ControlProperties, DesignCommand,
};
use datacraft_core::port::MockPort;
use datacraft_core::{ObjectId, ObjectKind};

fn opened_with_form() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core
}

fn label(name: &str, x: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: name.into() },
        rect: Rect {
            x,
            y: 8.0,
            width: 80.0,
            height: 24.0,
        },
        properties: ControlProperties::default(),
    }
}

fn three_controls(core: &mut Core) {
    for (name, x) in [("a", 8.0), ("b", 96.0), ("c", 184.0)] {
        core.dispatch(Command::AddFormControl {
            form: "Card".into(),
            control: label(name, x),
        });
    }
}

fn names(core: &Core) -> Vec<String> {
    core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|control| control.name.clone())
        .collect()
}

fn tab_order(core: &Core) -> Vec<(String, u32)> {
    let mut entries: Vec<(String, u32)> = core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|control| (control.name.clone(), control.properties.tab_index))
        .collect();
    entries.sort_by_key(|(_, index)| *index);
    entries
}

fn set_z(core: &mut Core, object: ObjectId, control: &str, position: usize) -> Vec<Event> {
    core.dispatch(Command::Design(DesignCommand::SetControlZOrder {
        object,
        control: control.to_owned(),
        position,
    }))
}

fn set_tab(core: &mut Core, object: ObjectId, control: &str, position: usize) -> Vec<Event> {
    core.dispatch(Command::Design(DesignCommand::SetControlTabOrder {
        object,
        control: control.to_owned(),
        position,
    }))
}

fn form_id() -> ObjectId {
    ObjectId::new(ObjectKind::Form, "Card")
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn placement_order_is_the_initial_paint_and_tab_order() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    assert_eq!(names(&core), ["a", "b", "c"]);
    assert_eq!(
        tab_order(&core),
        [
            ("a".to_owned(), 0),
            ("b".to_owned(), 1),
            ("c".to_owned(), 2)
        ],
        "a new control lands last in the tab order"
    );
}

#[test]
fn bringing_a_control_to_the_front_changes_the_paint_order() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    let events = set_z(&mut core, form_id(), "a", 2);
    assert_eq!(
        events,
        vec![Event::ControlOrderSet {
            object: form_id(),
            control: "a".into(),
            order: ControlOrder::Z,
            position: 2,
        }]
    );
    assert_eq!(names(&core), ["b", "c", "a"]);
    assert_eq!(
        tab_order(&core),
        [
            ("a".to_owned(), 0),
            ("b".to_owned(), 1),
            ("c".to_owned(), 2)
        ],
        "the tab order is untouched"
    );
}

#[test]
fn sending_a_control_to_the_back_changes_the_paint_order() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    set_z(&mut core, form_id(), "c", 0);
    assert_eq!(names(&core), ["c", "a", "b"]);
}

#[test]
fn an_out_of_range_z_position_is_clamped_to_the_last_slot() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    let events = set_z(&mut core, form_id(), "a", 99);
    assert_eq!(
        events,
        vec![Event::ControlOrderSet {
            object: form_id(),
            control: "a".into(),
            order: ControlOrder::Z,
            position: 2,
        }],
        "the event names the effective position"
    );
    assert_eq!(names(&core), ["b", "c", "a"]);
}

#[test]
fn the_tab_order_moves_a_control_and_renumbers_densely() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    let events = set_tab(&mut core, form_id(), "c", 0);
    assert_eq!(
        events,
        vec![Event::ControlOrderSet {
            object: form_id(),
            control: "c".into(),
            order: ControlOrder::Tab,
            position: 0,
        }]
    );
    assert_eq!(
        tab_order(&core),
        [
            ("c".to_owned(), 0),
            ("a".to_owned(), 1),
            ("b".to_owned(), 2)
        ]
    );
    assert_eq!(
        names(&core),
        ["a", "b", "c"],
        "the paint order is untouched"
    );
}

#[test]
fn a_tab_move_beyond_the_end_is_clamped() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    set_tab(&mut core, form_id(), "a", 99);
    assert_eq!(
        tab_order(&core),
        [
            ("b".to_owned(), 0),
            ("c".to_owned(), 1),
            ("a".to_owned(), 2)
        ]
    );
}

#[test]
fn z_order_and_tab_order_are_independent() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    set_z(&mut core, form_id(), "a", 2);
    set_tab(&mut core, form_id(), "a", 0);
    assert_eq!(names(&core), ["b", "c", "a"], "paint order");
    assert_eq!(
        tab_order(&core)
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"],
        "focus order"
    );
}

#[test]
fn report_controls_share_the_order_commands() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    for (name, x) in [("first", 8.0), ("second", 96.0)] {
        core.dispatch(Command::AddReportControl {
            report: "Weekly".into(),
            control: label(name, x),
        });
    }
    let report = ObjectId::new(ObjectKind::Report, "Weekly");
    let events = set_z(&mut core, report.clone(), "first", 1);
    assert_eq!(
        events,
        vec![Event::ControlOrderSet {
            object: report.clone(),
            control: "first".into(),
            order: ControlOrder::Z,
            position: 1,
        }]
    );
    set_tab(&mut core, report, "second", 0);
    let controls = core.inspect().database.unwrap().reports[0].controls.clone();
    assert_eq!(
        controls
            .iter()
            .map(|control| control.name.as_str())
            .collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert_eq!(controls[0].properties.tab_index, 0);
    assert_eq!(controls[1].properties.tab_index, 1);
}

#[test]
fn order_errors_are_precise() {
    let mut core = opened_with_form();
    three_controls(&mut core);
    let reason = failed_reason(&set_z(
        &mut core,
        ObjectId::new(ObjectKind::Form, "Ghost"),
        "a",
        1,
    ));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
    let reason = failed_reason(&set_z(&mut core, form_id(), "ghost", 1));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
    let reason = failed_reason(&set_tab(
        &mut core,
        ObjectId::new(ObjectKind::Table, "Card"),
        "a",
        1,
    ));
    let reason = reason.to_lowercase();
    assert!(
        reason.contains("form") && reason.contains("report"),
        "{reason}"
    );

    let mut closed = Core::new(Box::new(MockPort::new()));
    let events = set_tab(&mut closed, form_id(), "a", 0);
    assert!(matches!(
        events.as_slice(),
        [Event::Failed {
            code: datacraft_core::failure::FailureCode::NoDatabase,
            ..
        }]
    ));
}

#[test]
fn the_sidecar_keeps_the_tab_order() {
    use datacraft_core::schema::SchemaDocument;

    let mut form = datacraft_core::form::FormDef::new("Card");
    let mut first = label("a", 8.0);
    first.properties.tab_index = 1;
    let mut second = label("b", 96.0);
    second.properties.tab_index = 0;
    form.controls.push(first);
    form.controls.push(second);
    let mut doc = SchemaDocument::new("shop");
    doc.forms.push(form.clone());
    let back = SchemaDocument::from_json(&doc.to_json()).unwrap();
    assert_eq!(back.forms, vec![form]);
}
