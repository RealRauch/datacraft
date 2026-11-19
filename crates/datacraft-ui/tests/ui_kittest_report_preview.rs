//! The report preview contracts (step 91.11, items M123/L63): the
//! preview canvas follows the report's own PageSetup (the geometry and
//! the text scale ride the pure helpers), and the rendered page rect
//! rides the layout probe under `report.preview.page`.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;

/// App with an open report tab (the report tab's default view renders
/// the print preview).
fn preview_app() -> App {
    let port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![FieldDef::new("total", FieldKind::Currency)],
    )]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateReport {
        name: "Roster".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Report, "Roster", "Orders");
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Report,
        "Roster",
    )));
    app
}

/// The rendered preview page rides the layout probe.
#[test]
fn the_preview_page_records_its_rect() {
    let app = preview_app();
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::report_design::report_preview_ui(app, &tab, ui);
        },
        app,
    );
    harness.set_size(egui::vec2(900.0, 900.0));
    harness.run();
    let page = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "report.preview.page")
        .map(|(_, rect)| *rect)
        .expect("the preview page records its rect");
    // The A4 default at the shipped 0.9 scale.
    assert!(
        (page.width() - 595.0 * 0.9).abs() < 1.0,
        "the page renders at the A4 default: {page:?}"
    );
}
