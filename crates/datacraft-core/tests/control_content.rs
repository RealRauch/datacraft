//! Designer control content editing (step 63.1, item H56): every placed
//! control carries exactly one content property — the label text, button
//! caption, bound source field or embedded form — and the new
//! `SetFormControlContent`/`SetReportControlContent` commands set it with
//! precise validation. The runtimes/preview render the edited content.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

fn core() -> Core {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Text("alpha".to_owned()),
            FieldValue::Boolean(true),
        ]],
    );
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    core
}

fn control() -> ControlDef {
    ControlDef {
        name: "C1".into(),
        kind: ControlKind::Label {
            text: "default".into(),
        },
        rect: datacraft_core::canvas::Rect {
            x: 8.0,
            y: 8.0,
            width: 100.0,
            height: 24.0,
        },
    }
}

fn form_with_control(core: &mut Core) -> () {
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: control(),
    });
}

fn form_kind(core: &Core) -> ControlKind {
    core.inspect()
        .database
        .expect("open")
        .forms
        .into_iter()
        .find(|form| form.name == "Card")
        .expect("form")
        .controls
        .into_iter()
        .find(|control| control.name == "C1")
        .expect("control")
        .kind
}

/// The label text is settable after placement — the H56 gap: without
/// this, a placed control never shows what the user intends.
#[test]
fn a_form_label_text_is_editable() {
    let mut core = core();
    form_with_control(&mut core);
    let events = core.dispatch(Command::SetFormControlContent {
        form: "Card".into(),
        control: "C1".into(),
        value: "Hello".into(),
    });
    assert_eq!(
        events,
        vec![Event::ControlContentSet {
            form: "Card".into(),
            control: "C1".into(),
        }]
    );
    assert!(matches!(
        form_kind(&core),
        ControlKind::Label { text } if text == "Hello"
    ));
}

/// A binding answers precisely for unknown fields and clears on an empty
/// value; a valid field binds.
#[test]
fn a_form_binding_validates_against_the_source_and_clears() {
    let mut core = core();
    form_with_control(&mut core);
    core.dispatch(Command::SetFormControlContent {
        form: "Card".into(),
        control: "C1".into(),
        value: "x".into(),
    });
    // Turn the label into a text box by replacing the control.
    core.dispatch(Command::DeleteFormControl {
        form: "Card".into(),
        control: "C1".into(),
    });
    let mut bound = control();
    bound.kind = ControlKind::TextBox { bound_field: None };
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: bound,
    });

    let failed = core.dispatch(Command::SetFormControlContent {
        form: "Card".into(),
        control: "C1".into(),
        value: "missing".into(),
    });
    assert!(
        matches!(
            failed.as_slice(),
            [Event::Failed { code: datacraft_core::failure::FailureCode::InvalidInput, reason }]
                if reason.contains("missing")
        ),
        "{failed:?}"
    );

    core.dispatch(Command::SetFormControlContent {
        form: "Card".into(),
        control: "C1".into(),
        value: "name".into(),
    });
    assert!(matches!(
        form_kind(&core),
        ControlKind::TextBox {
            bound_field: Some(field)
        } if field == "name"
    ));
    core.dispatch(Command::SetFormControlContent {
        form: "Card".into(),
        control: "C1".into(),
        value: String::new(),
    });
    assert!(matches!(
        form_kind(&core),
        ControlKind::TextBox { bound_field: None }
    ));
}

/// A report label renders the edited text in the preview; a text box bound
/// to a field renders the record's value; a `=` expression stays allowed.
#[test]
fn a_report_control_content_renders_in_the_preview() {
    let mut core = core();
    core.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Sheet".into(),
        table: "Customers".into(),
    });
    let mut label = control();
    label.name = "Title".into();
    core.dispatch(Command::AddReportControl {
        report: "Sheet".into(),
        control: label,
    });
    core.dispatch(Command::SetReportControlContent {
        report: "Sheet".into(),
        control: "Title".into(),
        value: "Quarterly report".into(),
    });
    let mut bound = control();
    bound.name = "NameBox".into();
    bound.kind = ControlKind::TextBox { bound_field: None };
    bound.rect.y = 40.0;
    core.dispatch(Command::AddReportControl {
        report: "Sheet".into(),
        control: bound,
    });
    core.dispatch(Command::SetReportControlContent {
        report: "Sheet".into(),
        control: "NameBox".into(),
        value: "name".into(),
    });

    let pages = core.report_preview("Sheet").expect("the preview renders");
    let texts: Vec<&str> = pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.as_str()))
        .collect();
    assert!(
        texts.contains(&"Quarterly report"),
        "the edited label renders: {texts:?}"
    );
    assert!(
        texts.contains(&"alpha"),
        "the bound field resolves the record: {texts:?}"
    );
}

/// A report text box accepts a `=` expression (validated at render time)
/// while a binding still goes through the field whitelist.
#[test]
fn report_expressions_are_allowed_and_unknown_fields_are_not() {
    let mut core = core();
    core.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Sheet".into(),
        table: "Customers".into(),
    });
    let mut bound = control();
    bound.kind = ControlKind::TextBox { bound_field: None };
    core.dispatch(Command::AddReportControl {
        report: "Sheet".into(),
        control: bound,
    });
    let ok = core.dispatch(Command::SetReportControlContent {
        report: "Sheet".into(),
        control: "C1".into(),
        value: "=1+1".into(),
    });
    assert!(!matches!(ok.as_slice(), [Event::Failed { .. }]), "{ok:?}");

    let failed = core.dispatch(Command::SetReportControlContent {
        report: "Sheet".into(),
        control: "C1".into(),
        value: "ghost".into(),
    });
    assert!(
        matches!(
            failed.as_slice(),
            [Event::Failed {
                code: datacraft_core::failure::FailureCode::InvalidInput,
                ..
            }]
        ),
        "{failed:?}"
    );
}
