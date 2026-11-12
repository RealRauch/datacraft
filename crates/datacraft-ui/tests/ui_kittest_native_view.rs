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
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

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
            datacraft_ui::layout_probe::clear();
            let height = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(220.0, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| datacraft_ui::nav_pane::nav_pane_ui(app, ui),
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let Some(tab) = app.snapshot().tabs.first().cloned() else {
                            return;
                        };
                        datacraft_ui::views_window::view_ui(app, &tab, ui);
                    },
                );
            });
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
    // The grid paints its cells (no label nodes for body text), so the
    // data path is asserted on the cached result beside the painted row
    // selector.
    let tab = harness.state().snapshot().tabs[0].id;
    let result = harness
        .state()
        .view_result(tab)
        .expect("the native view result is cached");
    assert_eq!(result.columns, ["id", "name"]);
    assert_eq!(result.rows[0][1], FieldValue::Text("Ada".into()));
    assert!(
        dump.contains("     1"),
        "the read-only datasheet paints the row selector: {dump}"
    );
    let rects = datacraft_ui::layout_probe::drain();
    assert!(
        rects
            .iter()
            .any(|(id, rect)| id == "view.native_hint" && rect.width() > 0.0),
        "the tab names the boundary (rect probe): {rects:?}"
    );
    assert!(
        !dump.contains("Edit SQL") && !dump.contains("Delete"),
        "a native view offers no design or delete affordance: {dump}"
    );
}
