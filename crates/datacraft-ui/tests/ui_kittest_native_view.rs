//! Foreign (native) SQLite views in the UI (phase 73, step 73.1; item M90).
//!
//! The navigation pane lists the file's own views; opening one renders a
//! read-only datasheet with a boundary hint — no Edit SQL, no Delete, no
//! write affordance.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, NativeView};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn people() -> TableDef {
    TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn adults() -> NativeView {
    NativeView {
        name: "adults".into(),
        sql: "CREATE VIEW adults AS SELECT id, name FROM people".into(),
        fields: vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    }
}

fn native_view_app() -> App {
    let mut port = MockPort::with_tables(vec![people()]);
    port.with_native_views(vec![adults()]);
    port.with_row_data(
        "adults",
        vec![vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.sqlite".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::View,
        "adults",
    )));
    app
}

fn native_view_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::views_window::view_ui(app, &tab, ui);
        },
        app,
    )
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn a_native_view_is_listed_and_renders_read_only() {
    let mut harness = native_view_harness(native_view_app());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("adults"),
        "the nav pane lists the foreign view: {dump}"
    );
    assert!(
        dump.contains("Ada"),
        "the read-only datasheet renders the view rows: {dump}"
    );
    assert!(
        dump.contains("Read-only native view"),
        "the tab names the boundary: {dump}"
    );
    assert!(
        !dump.contains("Edit SQL") && !dump.contains("Delete"),
        "a native view offers no design or delete affordance: {dump}"
    );
}
