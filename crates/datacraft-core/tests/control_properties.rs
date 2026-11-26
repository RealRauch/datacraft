//! Contract suite for the placed-control property model (phase 102, item
//! M133): one command per property edit, core-owned validation that leaves
//! the design unchanged on failure, and sidecar persistence.
//!
//! The property set is shared by both designers — forms and reports carry
//! the same [`ControlDef`] — so the command addresses the owning object.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{
    ControlDef, ControlKind, ControlProperties, ControlProperty, SetControlProperties, TextAlign,
};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
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

fn label(name: &str, text: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: text.into() },
        rect: Rect {
            x: 8.0,
            y: 8.0,
            width: 120.0,
            height: 24.0,
        },
        properties: ControlProperties::default(),
    }
}

fn form_control(core: &Core, form: &str, control: &str) -> ControlDef {
    let db = core.inspect().database.expect("open");
    db.forms
        .iter()
        .find(|f| f.name == form)
        .expect("form")
        .controls
        .iter()
        .find(|c| c.name == control)
        .expect("control")
        .clone()
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

fn failed_code(events: &[Event]) -> datacraft_core::failure::FailureCode {
    match events {
        [Event::Failed { code, .. }] => *code,
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

fn set(core: &mut Core, control: &str, property: ControlProperty) -> Vec<Event> {
    core.dispatch(Command::SetControlProperties(SetControlProperties {
        object: ObjectId::new(ObjectKind::Form, "Card"),
        control: control.to_owned(),
        property,
    }))
}

#[test]
fn placed_controls_start_visible_enabled_and_left_aligned() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers"),
    });
    assert_eq!(
        form_control(&core, "Card", "title").properties,
        ControlProperties::default()
    );
    let properties = form_control(&core, "Card", "title").properties;
    assert!(properties.visible);
    assert!(properties.enabled);
    assert!(!properties.bold);
    assert_eq!(properties.align, TextAlign::Left);
}

#[test]
fn one_command_sets_exactly_one_style_property() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers"),
    });

    let events = set(&mut core, "title", ControlProperty::Bold(true));
    assert_eq!(
        events,
        vec![Event::ControlPropertiesSet {
            object: ObjectId::new(ObjectKind::Form, "Card"),
            control: "title".into(),
        }]
    );
    let properties = form_control(&core, "Card", "title").properties;
    assert!(properties.bold, "bold landed");
    assert!(properties.visible, "the other properties stayed");
    assert_eq!(properties.align, TextAlign::Left);

    set(&mut core, "title", ControlProperty::Visible(false));
    set(&mut core, "title", ControlProperty::Enabled(false));
    set(&mut core, "title", ControlProperty::Align(TextAlign::Right));
    let properties = form_control(&core, "Card", "title").properties;
    assert!(!properties.visible);
    assert!(!properties.enabled);
    assert!(properties.bold, "earlier edits survive later ones");
    assert_eq!(properties.align, TextAlign::Right);
}

#[test]
fn renaming_a_control_keeps_the_namespace_unique() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("first", "One"),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("second", "Two"),
    });

    let events = set(&mut core, "first", ControlProperty::Name("Title".into()));
    assert_eq!(
        events,
        vec![Event::ControlPropertiesSet {
            object: ObjectId::new(ObjectKind::Form, "Card"),
            control: "Title".into(),
        }],
        "the event carries the new canonical name"
    );
    let names: Vec<String> = core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(names, vec!["Title".to_owned(), "second".to_owned()]);

    // A duplicate (case-insensitive) is refused and changes nothing.
    let events = set(&mut core, "second", ControlProperty::Name("title".into()));
    assert_eq!(failed_code(&events), datacraft_core::failure::FailureCode::Duplicate);
    assert!(failed_reason(&events).to_lowercase().contains("title"));
    assert_eq!(
        form_control(&core, "Card", "second").name,
        "second",
        "the refused rename left the control untouched"
    );
}

#[test]
fn an_empty_or_blank_control_name_is_refused() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers"),
    });
    for blank in ["", "   ", "\t"] {
        let events = set(&mut core, "title", ControlProperty::Name(blank.into()));
        assert_eq!(
            failed_code(&events),
            datacraft_core::failure::FailureCode::InvalidInput,
            "blank name {blank:?} must be refused"
        );
        assert_eq!(form_control(&core, "Card", "title").name, "title");
    }
}

#[test]
fn a_surrounding_whitespace_name_is_trimmed() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers"),
    });
    set(&mut core, "title", ControlProperty::Name("  Heading  ".into()));
    let names: Vec<String> = core.inspect().database.unwrap().forms[0]
        .controls
        .iter()
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(names, vec!["Heading".to_owned()]);
}

#[test]
fn report_controls_share_the_property_command() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("Heading", "Sales"),
    });
    let events = core.dispatch(Command::SetControlProperties(SetControlProperties {
        object: ObjectId::new(ObjectKind::Report, "Weekly"),
        control: "Heading".into(),
        property: ControlProperty::Align(TextAlign::Center),
    }));
    assert_eq!(
        events,
        vec![Event::ControlPropertiesSet {
            object: ObjectId::new(ObjectKind::Report, "Weekly"),
            control: "Heading".into(),
        }]
    );
    let db = core.inspect().database.unwrap();
    assert_eq!(
        db.reports[0].controls[0].properties.align,
        TextAlign::Center
    );
}

#[test]
fn other_object_kinds_answer_precisely() {
    let mut core = opened_with_form();
    let events = core.dispatch(Command::SetControlProperties(SetControlProperties {
        object: ObjectId::new(ObjectKind::Table, "Card"),
        control: "title".into(),
        property: ControlProperty::Bold(true),
    }));
    assert_eq!(
        failed_code(&events),
        datacraft_core::failure::FailureCode::InvalidInput
    );
    let reason = failed_reason(&events).to_lowercase();
    assert!(
        reason.contains("form") && reason.contains("report"),
        "the refusal names the object kinds that carry controls: {reason}"
    );
}

#[test]
fn unknown_objects_and_controls_answer_not_found() {
    let mut core = opened_with_form();
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("title", "Customers"),
    });
    let reason = failed_reason(&core.dispatch(Command::SetControlProperties(
        SetControlProperties {
            object: ObjectId::new(ObjectKind::Form, "Ghost"),
            control: "title".into(),
            property: ControlProperty::Bold(true),
        },
    )));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");

    let reason = failed_reason(&core.dispatch(Command::SetControlProperties(
        SetControlProperties {
            object: ObjectId::new(ObjectKind::Form, "Card"),
            control: "ghost".into(),
            property: ControlProperty::Bold(true),
        },
    )));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn every_property_edit_requires_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::SetControlProperties(SetControlProperties {
        object: ObjectId::new(ObjectKind::Form, "Card"),
        control: "title".into(),
        property: ControlProperty::Bold(true),
    }));
    assert_eq!(
        failed_code(&events),
        datacraft_core::failure::FailureCode::NoDatabase
    );
}

#[test]
fn properties_survive_the_sidecar_round_trip() {
    use datacraft_core::schema::SchemaDocument;

    let mut form = datacraft_core::form::FormDef::new("Card");
    let mut control = label("title", "Customers");
    control.properties = ControlProperties {
        visible: false,
        enabled: false,
        bold: true,
        align: TextAlign::Center,
    };
    form.controls.push(control);

    let mut doc = SchemaDocument::new("shop");
    doc.forms.push(form.clone());
    let back = SchemaDocument::from_json(&doc.to_json()).unwrap();
    assert_eq!(back.forms, vec![form]);
}

#[test]
fn an_older_sidecar_without_properties_loads_the_defaults() {
    use datacraft_core::schema::SchemaDocument;

    // A pre-phase-102 sidecar: controls carry only name, kind and rect.
    let json = r#"{
        "schema_version": 1,
        "database_name": "shop",
        "tables": [],
        "forms": [{
            "name": "Card",
            "source": "",
            "grid": 8.0,
            "size": { "width": 640.0, "height": 400.0 },
            "controls": [{
                "name": "title",
                "kind": { "type": "label", "value": { "text": "Customers" } },
                "rect": { "x": 8.0, "y": 8.0, "width": 120.0, "height": 24.0 }
            }]
        }]
    }"#;
    let document = SchemaDocument::from_json(json).expect("older sidecar loads");
    assert_eq!(
        document.forms[0].controls[0].properties,
        ControlProperties::default(),
        "missing properties fall back to the defaults"
    );
}

fn table_with_one_row() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )
}

fn opened_with_report() -> Core {
    let mut port = MockPort::with_tables(vec![table_with_one_row()]);
    port.with_row_data(
        "Customers",
        vec![vec![FieldValue::Text("alpha".to_owned())]],
    );
    let mut core = Core::new(Box::new(port));
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
    core
}

#[test]
fn the_preview_carries_the_style_and_omits_hidden_controls() {
    let mut core = opened_with_report();
    let mut shown = label("Heading", "Sales");
    shown.properties.bold = true;
    shown.properties.align = TextAlign::Center;
    let mut hidden = label("Secret", "Internal");
    hidden.properties.visible = false;
    hidden.rect.y = 48.0;
    for control in [shown, hidden] {
        core.dispatch(Command::AddReportControl {
            report: "Weekly".into(),
            control,
        });
    }

    let pages = core.report_preview("Weekly").expect("preview");
    let items = &pages[0].items;
    assert_eq!(items.len(), 1, "the hidden control renders nothing: {items:?}");
    assert_eq!(items[0].text, "Sales");
    assert!(items[0].bold);
    assert_eq!(items[0].align, TextAlign::Center);
}
