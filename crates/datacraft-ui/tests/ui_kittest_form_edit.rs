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
use datacraft_core::form::{ButtonAction, ControlDef, ControlKind, FormFilter};
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

/// App with a form over three bound text controls plus the `Id` text box
/// (its Number kind makes a mistyped value observable) and the `Active`
/// checkbox (row 1 carries `Null`).
fn form_app() -> (App, TabId) {
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

fn cached_value(
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

#[test]
fn a_checkbox_click_toggles_true_to_false_and_writes_with_the_record() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "ActiveBox")
        .click();
    harness.run();
    assert_eq!(
        harness.state().snapshot().undo_depth,
        0,
        "the click stages the value, it does not write"
    );

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached_value(&harness, tab, 0, 3),
        Some(FieldValue::Boolean(false)),
        "leaving the record writes the toggled value"
    );
}

#[test]
fn a_null_checkbox_shows_unchecked_and_a_click_writes_true() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Next >").click();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "ActiveBox")
        .click();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness.get_by_label("< Prev").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached_value(&harness, tab, 1, 3),
        Some(FieldValue::Boolean(true)),
        "a Null checkbox displays as No and the click writes Yes (Access TripleState=False)"
    );
}

#[test]
fn a_combo_selection_comes_from_the_row_source_rows() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "NameCombo values")
        .click();
    harness.run();
    harness.get_by_label("mika@example.com").click();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(
        cached(&harness, tab, 0, 1),
        "mika@example.com",
        "the selected display value is written to the bound field"
    );
}

#[test]
fn a_combo_accepts_free_text() {
    let (app, tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    click_input(&mut harness, "NameCombo");
    type_into(&mut harness, "X");
    harness.get_by_label("Next >").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 1);
    assert_eq!(cached(&harness, tab, 0, 1), "LenaX");
}

fn displayed(harness: &Harness<'static, App>, name: &str) -> String {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, name)
        .accesskit_node()
        .value()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_form_filter_narrows_the_record_set_and_clears() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();
    assert_eq!(displayed(&harness, "NameBox"), "Lena");

    harness
        .state_mut()
        .dispatch(Command::SetFormFilter(FormFilter {
            form: "Form1".into(),
            criteria: Some("[Name] Like 'L*'".into()),
        }));
    harness.run();
    assert!(
        tree_contains(&harness, "Record 1 of 2"),
        "the filtered count drives the navigation box"
    );
    assert_eq!(displayed(&harness, "NameBox"), "Lena");

    harness.get_by_label("Next >").click();
    harness.run();
    assert!(tree_contains(&harness, "Record 2 of 2"));
    assert_eq!(
        displayed(&harness, "NameBox"),
        "Lars",
        "navigation stays inside the filtered set"
    );

    harness.get_by_label("Clear filter").click();
    harness.run();
    assert!(
        tree_contains(&harness, "Record 3 of 3"),
        "clearing keeps the current record visible (Access semantics)"
    );
    assert_eq!(displayed(&harness, "NameBox"), "Lars");
}

fn tree_contains(harness: &Harness<'static, App>, needle: &str) -> bool {
    harness.root().children_recursive().any(|node| {
        let node = node.accesskit_node();
        node.label().is_some_and(|label| label.contains(needle))
            || node.value().is_some_and(|value| value.contains(needle))
    })
}

#[test]
fn a_button_with_an_action_dispatches_one_command() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();
    assert_eq!(harness.state().snapshot().tabs.len(), 1);

    harness.get_by_label("Open").click();
    harness.run();
    harness.run();

    let snapshot = harness.state().snapshot();
    assert_eq!(snapshot.tabs.len(), 2, "one OpenObject dispatch opened Second");
    assert_eq!(snapshot.tabs[1].object_name, "Second");
}

#[test]
fn a_button_without_an_action_reports_precisely() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Empty").click();
    harness.run();

    assert_eq!(
        harness.state().snapshot().tabs.len(),
        1,
        "the empty action dispatches nothing"
    );
    let error = harness
        .state()
        .last_error()
        .expect("the empty action reports");
    assert!(error.contains("action"), "{error}");
}

#[test]
fn a_filter_button_narrows_the_form() {
    let (app, _tab) = form_app();
    let mut harness = form_harness(app);
    harness.run();

    harness.get_by_label("Filter").click();
    harness.run();
    harness.run();
    assert!(
        tree_contains(&harness, "Record 1 of 2"),
        "the button applied the form filter"
    );
}
