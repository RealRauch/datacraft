//! Read-only index/trigger inventory in the design view (phase 73, step
//! 73.2; item M90).
//!
//! The table design view names the file's own indexes and triggers below
//! the field property sheet. The section is paint-only (no command, no
//! affordance); the data itself is asserted on the snapshot.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, SourceIndex, SourceTrigger};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind, state::TabMode};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;

fn people() -> TableDef {
    TableDef::new(
        "people",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("city", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn design_app(with_objects: bool) -> App {
    let mut port = MockPort::with_tables(vec![people()]);
    if with_objects {
        port.with_indexes(vec![SourceIndex {
            table: "people".into(),
            name: "idx_people_city".into(),
            columns: vec!["city".into()],
            unique: false,
            automatic: false,
        }]);
        port.with_triggers(vec![SourceTrigger {
            table: "people".into(),
            name: "people_guard".into(),
            sql: "CREATE TRIGGER people_guard AFTER INSERT ON people BEGIN SELECT 1; END".into(),
        }]);
    }
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.sqlite".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Table, "people"),
        mode: TabMode::Design,
    });
    app
}

fn design_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::layout_probe::clear();
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                datacraft_ui::design::design_ui(app, &tab, ui);
            });
        },
        app,
    )
}

#[test]
fn the_design_view_paints_the_file_object_inventory() {
    let mut harness = design_harness(design_app(true));
    harness.run();
    let rects = datacraft_ui::layout_probe::drain();
    assert!(
        rects
            .iter()
            .any(|(id, rect)| id == "design.file_objects" && rect.width() > 0.0),
        "the inventory section is painted: {rects:?}"
    );
    let database = harness.state().snapshot().database.expect("open");
    assert_eq!(database.indexes[0].name, "idx_people_city");
    assert_eq!(database.triggers[0].name, "people_guard");
}

#[test]
fn a_table_without_file_objects_paints_no_inventory() {
    let mut harness = design_harness(design_app(false));
    harness.run();
    let rects = datacraft_ui::layout_probe::drain();
    assert!(
        !rects.iter().any(|(id, _)| id == "design.file_objects"),
        "no inventory section for a clean table: {rects:?}"
    );
}
