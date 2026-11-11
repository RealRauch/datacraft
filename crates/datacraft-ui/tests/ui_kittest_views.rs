//! View tab and editor suite (step 71.2, item M93).
//!
//! A view tab renders its SELECT result read-only; the editor creates and
//! updates sidecar views through the bus.

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::view::ViewCommand;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn app_with_rows() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Lena".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Mara".into())],
            vec![FieldValue::Integer(3), FieldValue::Text("Tom".into())],
        ],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut text = String::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        if label.is_empty() {
            text.push_str(&node.value().unwrap_or_default());
        } else {
            text.push_str(&label);
        }
        text.push('|');
    }
    text
}

/// A view tab renders the SELECT rows read-only.
#[test]
fn a_view_tab_renders_its_select_result() {
    let mut app = app_with_rows();
    app.dispatch(Command::View(ViewCommand::Create {
        name: "All".into(),
        sql: "SELECT Name FROM Customers".into(),
    }));
    app.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::View, "All")));

    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::views_window::view_ui(app, &tab, ui);
        },
        app,
    );
    harness.run();

    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Lena") && dump.contains("Mara") && dump.contains("Tom"),
        "the view shows its result rows: {dump}"
    );
}

/// The editor creates a view and later replaces its SQL.
#[test]
fn the_view_editor_creates_and_updates_a_view() {
    let mut app = app_with_rows();
    app.open_view_editor(None);
    app.set_view_draft_name("All");
    app.set_view_draft_sql("SELECT 1 AS one");
    app.save_view_draft();
    let db = app.snapshot().database.clone().expect("open");
    assert_eq!(db.views.len(), 1);
    assert_eq!(db.views[0].name, "All");
    assert_eq!(db.views[0].sql, "SELECT 1 AS one");

    app.open_view_editor(Some("All"));
    app.set_view_draft_sql("SELECT 2 AS two");
    app.save_view_draft();
    let db = app.snapshot().database.clone().expect("open");
    assert_eq!(db.views[0].sql, "SELECT 2 AS two");
}
