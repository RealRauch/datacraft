//! The form record navigation row (step 91.12, item M124): first/last
//! jump and the new-record insert ride the row — capability-gated like
//! the datasheet's row actions, with the edit buffer flushed exactly
//! like prev/next. The insert flow pins the immediate-write driver
//! (atomic_batch off): the new record lands in the row set and the
//! view navigates to it.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn form_app(atomic_batch: bool) -> App {
    let mut port = MockPort::with_tables(vec![customers_table()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Lena".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Mara".into())],
            vec![FieldValue::Integer(3), FieldValue::Text("Tom".into())],
        ],
    );
    if !atomic_batch {
        let mut caps = DriverCapabilities::full();
        caps.atomic_batch = false;
        port.with_capabilities(caps);
    }
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Form1".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Form, "Form1", "Customers");
    app.dispatch(Command::AddFormControl {
        form: "Form1".into(),
        control: datacraft_core::form::ControlDef {
            name: "NameBox".into(),
            kind: datacraft_core::form::ControlKind::TextBox {
                bound_field: Some("Name".into()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 8.0,
                y: 8.0,
                width: 160.0,
                height: 24.0,
            },
            properties: datacraft_core::form::ControlProperties::default(),
        },
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Form1",
    )));
    app
}

fn runtime_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
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

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().map(|c| c.to_string());
        let value = node.value().map(|c| c.to_string());
        let text = label.or(value).unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{text}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// First and last jump to the ends of the row set (M124).
#[test]
fn first_and_last_jump_the_record() {
    let mut harness = runtime_harness(form_app(true));
    harness.run();
    harness.get_by_label("Last").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Record 3 of 3"),
        "Last jumps to the end:\n{dump}"
    );
    harness.get_by_label("First").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Record 1 of 3"),
        "First jumps to the start:\n{dump}"
    );
}

/// The new-record action inserts through the normal path and navigates
/// to the new last record (the immediate-write driver pins the row-set
/// growth).
#[test]
fn the_new_record_action_inserts_and_navigates() {
    let mut harness = runtime_harness(form_app(false));
    harness.run();
    harness.get_by_label("New record").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Record 4 of 4"),
        "the new record lands and the view navigates to it:\n{dump}"
    );
}

/// The new-record action respects the driver's insert capability
/// (capability-gated like the datasheet's row actions).
#[test]
fn the_new_record_action_respects_the_insert_capability() {
    let mut port = MockPort::with_tables(vec![customers_table()]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Integer(1),
            FieldValue::Text("Lena".into()),
        ]],
    );
    let mut caps = DriverCapabilities::full();
    caps.write_rows = false;
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Form1".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Form, "Form1", "Customers");
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Form1",
    )));
    let mut harness = runtime_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.lines()
            .any(|line| line.starts_with("Button|New record") && line.ends_with(" disabled")),
        "the new-record action renders disabled on a read-only source:\n{dump}"
    );
}
