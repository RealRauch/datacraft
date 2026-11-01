//! Contract suite for the control palette and placement (step 8.2, H29).
//!
//! Controls are placed with snapped, bounds-checked rects; names are
//! unique per form (case-insensitive); the palette mirrors the Access
//! toolbox with display names computed in core.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlDef, ControlKind, FormDef};
use datacraft_core::port::MockPort;
use datacraft_core::present::control_kind_display;

fn opened_with_form() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase { path: PathBuf::from("shop.mdb") });
    core.dispatch(Command::CreateForm { name: "Card".into() });
    core
}

fn label(name: &str, text: &str, x: f32, y: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label {
            text: text.into(),
        },
        rect: Rect { x, y, width: 100.0, height: 24.0 },
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn adding_a_control_snaps_and_is_observable() {
    let mut core = opened_with_form();
    let events = core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers", 37.0, 5.0),
    });
    assert_eq!(
        events,
        vec![Event::ControlAdded {
            form: "Card".into(),
            control: "title".into(),
        }]
    );
    let form = core.inspect().database.unwrap().forms[0].clone();
    assert_eq!(form.controls[0].rect, Rect { x: 40.0, y: 8.0, width: 100.0, height: 24.0 });
}

#[test]
fn control_names_are_unique_per_form() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "One", 0.0, 0.0),
    });
    let reason = failed_reason(&core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("TITLE", "Two", 0.0, 40.0),
    }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn controls_must_stay_inside_the_canvas() {
    let mut core = opened_with_form();
    let mut control = label("big", "Customers", 600.0, 390.0);
    control.rect.width = 100.0;
    control.rect.height = 24.0;
    let reason = failed_reason(&core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control,
    }));
    assert!(reason.to_lowercase().contains("bounds"), "{reason}");
}

#[test]
fn moving_a_control_snaps_and_updates_the_snapshot() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers", 0.0, 0.0),
    });
    core.dispatch(Command::SetFormControlRect {
        form: "Card".into(),
        control: "title".into(),
        rect: Rect { x: 61.0, y: 23.0, width: 100.0, height: 24.0 },
    });
    let form = core.inspect().database.unwrap().forms[0].clone();
    assert_eq!(form.controls[0].rect.x, 64.0);
    assert_eq!(form.controls[0].rect.y, 24.0);

    let reason = failed_reason(&core.dispatch(Command::SetFormControlRect {
        form: "Card".into(),
        control: "ghost".into(),
        rect: Rect { x: 0.0, y: 0.0, width: 10.0, height: 10.0 },
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn deleting_a_control_is_observable() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers", 0.0, 0.0),
    });
    assert_eq!(
        core.dispatch(Command::DeleteFormControl {
            form: "Card".into(),
            control: "title".into(),
        }),
        vec![Event::ControlDropped {
            form: "Card".into(),
            control: "title".into(),
        }]
    );
    assert!(core.inspect().database.unwrap().forms[0].controls.is_empty());
}

#[test]
fn forms_carry_a_canvas_size_with_defaults() {
    let form = FormDef::new("Card");
    assert_eq!(form.size.width, 640.0);
    assert_eq!(form.size.height, 400.0);
}

#[test]
fn the_palette_displays_access_toolbox_names() {
    assert_eq!(control_kind_display(&ControlKind::Label { text: "x".into() }), "Label");
    assert_eq!(
        control_kind_display(&ControlKind::TextBox { bound_field: None }),
        "Text Box"
    );
    assert_eq!(
        control_kind_display(&ControlKind::CheckBox { bound_field: None }),
        "Check Box"
    );
    assert_eq!(
        control_kind_display(&ControlKind::ComboBox { bound_field: None }),
        "Combo Box"
    );
    assert_eq!(
        control_kind_display(&ControlKind::Button { caption: "x".into() }),
        "Button"
    );
    assert_eq!(
        control_kind_display(&ControlKind::SubForm { form: "Card".into() }),
        "Subform"
    );
}

#[test]
fn controls_survive_the_sidecar_round_trip() {
    use datacraft_core::schema::SchemaDocument;
    let mut form = FormDef::new("Card");
    form.controls.push(ControlDef {
        name: "name_box".into(),
        kind: ControlKind::TextBox {
            bound_field: Some("name".into()),
        },
        rect: Rect { x: 120.0, y: 40.0, width: 160.0, height: 24.0 },
    });
    let mut doc = SchemaDocument::new("shop");
    doc.forms.push(form.clone());
    let back = SchemaDocument::from_json(&doc.to_json()).unwrap();
    assert_eq!(back.forms, vec![form]);
}
