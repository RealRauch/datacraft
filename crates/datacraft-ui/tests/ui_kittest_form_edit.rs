//! Form runtime bound-field editing suite (phase 64, M68; ADR-8).
//!
//! Typing edits a per-record buffer; exactly one `UpdateRow` is dispatched
//! when the record is left, never per keystroke or focus move. `Esc` once
//! reverts the field, `Esc` twice the record. A mistyped value keeps the
//! buffer and reports precisely.

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind};
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

/// App with a form over two bound controls plus the `Id` text box (its
/// Number kind makes a mistyped value observable).
fn form_app() -> (App, TabId) {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![
                FieldValue::Integer(1),
                FieldValue::Text("Lena".to_owned()),
                FieldValue::Text("lena@example.com".to_owned()),
            ],
            vec![
                FieldValue::Integer(2),
                FieldValue::Text("Mika".to_owned()),
                FieldValue::Text("mika@example.com".to_owned()),
            ],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Form1".into(),
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
    app.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Form,
        "Form1",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

fn form_harness(app: App) -> Harness<'static, App> {
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

fn cached(harness: &Harness<'static, App>, tab: TabId, row: usize, col: usize) -> String {
    harness
        .state()
        .rows_for(tab)
        .and_then(|rows| rows.get(row))
        .and_then(|values| values.get(col))
        .map(datacraft_ui::labels::value_label)
        .unwrap_or_default()
}

/// Clicks the editable text input named after its control (accessibility
/// name set by the runtime).
fn click_input(harness: &mut Harness<'static, App>, name: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, name)
        .click();
    harness.run();
}

/// Types into the focused input and confirms the control value with Enter
/// (the record buffer keeps it; nothing is written yet).
fn type_into(harness: &mut Harness<'static, App>, text: &str) {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
}

#[test]
fn an_edit_is_buffered_until_the_record_is_left() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "X");
    eprintln!(
        "enter: pending={} err={:?} undo={}",
        harness.state().form_has_pending(tab),
        harness.state().last_error(),
        harness.state().snapshot().undo_depth
    );
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "Enter updates the control, not the database"
    );

    harness.get_by_label("Next >").click();
    harness.run();
    eprintln!(
        "next1: pending={} err={:?} undo={} rec={}",
        harness.state().form_has_pending(tab),
        harness.state().last_error(),
        harness.state().snapshot().undo_depth,
        harness.state().form_record(tab)
    );
    harness.run();
    eprintln!(
        "next2: pending={} err={:?} undo={} rec={}",
        harness.state().form_has_pending(tab),
        harness.state().last_error(),
        harness.state().snapshot().undo_depth,
        harness.state().form_record(tab)
    );

    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "leaving the record dispatches exactly one UpdateRow"
    );
    assert_eq!(
        cached(&harness, tab, 0, 1),
        "LenaX",
        "the record change writes the buffered value"
    );
}

#[test]
fn escape_once_reverts_the_field_and_twice_the_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    // Two fields buffered on the same record.
    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "X");
    click_input(&mut harness, "EmailBox");
    type_into(&mut harness, "Y");

    // First Esc reverts the focused field only.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();

    // Leaving the record now writes nothing for either field.
    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "Esc twice discards the whole record"
    );
    assert_eq!(cached(&harness, tab, 0, 1), "Lena");
    assert_eq!(cached(&harness, tab, 0, 2), "lena@example.com");
}

#[test]
fn a_mistyped_value_keeps_the_buffer_and_reports_precisely() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "IdBox");
    type_into(&mut harness, "abc");

    let error = harness
        .state()
        .last_error()
        .expect("the failed parse reports")
        .to_owned();
    assert!(error.contains("Id"), "the failure names the field: {error}");
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    // The buffer survives: the value still renders the typed text.
    let values: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let value = node.accesskit_node().value().unwrap_or_default();
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect();
    assert!(
        values.iter().any(|value| value == "1abc"),
        "the buffered text stays visible: {values:?}"
    );
    assert_eq!(cached(&harness, tab, 0, 0), "1");
}

#[test]
fn closing_the_tab_writes_the_buffered_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameBox");
    type_into(&mut harness, "Z");
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness
        .state_mut()
        .dispatch(Command::CloseTab { tab_id: tab });
    assert_eq!(
        harness.state().snapshot().undo_depth,
        1,
        "closing the tab saves the pending record"
    );
    assert!(
        harness.state().last_error().is_none(),
        "the save on close succeeds"
    );
}
