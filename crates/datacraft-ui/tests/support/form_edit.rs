//! Shared helpers for the form-runtime kittest suites (phase 64, ADR-8).
//! Every binary includes the module and uses a subset — dead code is
//! allowed, like the headless E2E support module.

#![allow(dead_code)]

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::form::{ButtonAction, ControlDef, ControlKind};
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Email", FieldKind::Text { max_len: 80 }),
            FieldDef::new("Active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

fn text_box(name: &str, bound: &str, x: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::TextBox {
            bound_field: Some(bound.into()),
        },
        rect: Rect {
            x,
            y: 48.0,
            width: 180.0,
            height: 24.0,
        },
    }
}

fn combo_box(name: &str, bound: &str, row_source: &str, display_field: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::ComboBox {
            bound_field: Some(bound.into()),
            row_source: row_source.into(),
            display_field: display_field.into(),
        },
        rect: Rect {
            x: 32.0,
            y: 136.0,
            width: 180.0,
            height: 24.0,
        },
    }
}

fn button(name: &str, caption: &str, action: Option<ButtonAction>, x: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Button {
            caption: caption.into(),
            action,
        },
        rect: Rect {
            x,
            y: 176.0,
            width: 96.0,
            height: 24.0,
        },
    }
}

fn check_box(name: &str, bound: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::CheckBox {
            bound_field: Some(bound.into()),
        },
        rect: Rect {
            x: 32.0,
            y: 96.0,
            width: 20.0,
            height: 20.0,
        },
    }
}

/// App with a form over three bound text controls, the `Id` text box (its
/// Number kind makes a mistyped value observable), the `Active` checkbox
/// (row 1 carries `Null`), a combo and three action buttons.
pub fn form_app() -> (App, TabId) {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("Lena".to_owned()),
                FieldValue::Text("lena@example.com".to_owned()),
                FieldValue::Boolean(true),
            ],
            vec![
                FieldValue::Integer(2),
                FieldValue::Text("Mika".to_owned()),
                FieldValue::Text("mika@example.com".to_owned()),
                FieldValue::Null,
            ],
            vec![
                FieldValue::Integer(3),
                FieldValue::Text("Lars".to_owned()),
                FieldValue::Text("lars@example.com".to_owned()),
                FieldValue::Boolean(false),
            ],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Form1".into(),
    });
    app.dispatch(Command::CreateForm {
        name: "Second".into(),
    });
    app.set_record_source(ObjectKind::Form, "Form1", "Customers");
    for (name, bound, x) in [
        ("NameBox", "Name", 32.0),
        ("EmailBox", "Email", 240.0),
        ("IdBox", "Id", 448.0),
    ] {
        app.dispatch(Command::AddFormControl {
            form: "Form1".into(),
            control: text_box(name, bound, x),
        });
    }
    app.dispatch(Command::AddFormControl {
        form: "Form1".into(),
        control: check_box("ActiveBox", "Active"),
    });
    app.dispatch(Command::AddFormControl {
        form: "Form1".into(),
        control: combo_box("NameCombo", "Name", "Customers", "Email"),
    });
    for control in [
        button(
            "OpenButton",
            "Open",
            Some(ButtonAction::OpenForm {
                form: "Second".into(),
            }),
            32.0,
        ),
        button(
            "FilterButton",
            "Filter",
            Some(ButtonAction::ApplyFilter {
                criteria: "[Name] Like 'L*'".into(),
            }),
            160.0,
        ),
        button("EmptyButton", "Empty", None, 288.0),
    ] {
        app.dispatch(Command::AddFormControl {
            form: "Form1".into(),
            control,
        });
    }
    app.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Form,
        "Form1",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

pub fn form_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::form_runtime::form_runtime_ui(app, &tab, ui);
        },
        app,
    )
}

pub fn cached(harness: &Harness<'static, App>, tab: TabId, row: usize, col: usize) -> String {
    harness
        .state()
        .rows_for(tab)
        .and_then(|rows| rows.get(row))
        .and_then(|values| values.get(col))
        .map(datacraft_ui::labels::value_label)
        .unwrap_or_default()
}

pub fn cached_value(
    harness: &Harness<'static, App>,
    tab: TabId,
    row: usize,
    col: usize,
) -> Option<FieldValue> {
    harness
        .state()
        .rows_for(tab)
        .and_then(|rows| rows.get(row))
        .and_then(|values| values.get(col))
        .cloned()
}

/// Clicks the editable text input named after its control (accessibility
/// name set by the runtime).
pub fn click_input(harness: &mut Harness<'static, App>, name: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, name)
        .click();
    harness.run();
}

/// Types into the focused input and confirms the control value with Enter
/// (the record buffer keeps it; nothing is written yet).
pub fn type_into(harness: &mut Harness<'static, App>, text: &str) {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
}

pub fn displayed(harness: &Harness<'static, App>, name: &str) -> String {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, name)
        .accesskit_node()
        .value()
        .unwrap_or_default()
        .to_owned()
}

pub fn tree_contains(harness: &Harness<'static, App>, needle: &str) -> bool {
    harness.root().children_recursive().any(|node| {
        let node = node.accesskit_node();
        node.label().is_some_and(|label| label.contains(needle))
            || node.value().is_some_and(|value| value.contains(needle))
    })
}
