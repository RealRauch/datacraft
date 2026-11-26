//! The print preview's zoom control (phase 102, step 102.4; item M133):
//! the hard-coded 0.9 became a per-surface setting — choosing a zoom
//! repaints the page at that scale and persists in `ui.json`. The setting
//! contract (round trip, sanitizing) is pinned by the settings unit test.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn report_app(settings_path: std::path::PathBuf) -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![FieldValue::Text("alpha".to_owned())]],
    );
    let mut app = App::new(Box::new(port));
    app.set_settings_path(Some(settings_path));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    app.dispatch(Command::SetReportSource {
        report: "Sheet".into(),
        table: "Customers".into(),
    });
    app.dispatch(datacraft_core::command::Command::AddReportControl {
        report: "Sheet".into(),
        control: datacraft_core::form::ControlDef {
            name: "Title".into(),
            kind: datacraft_core::form::ControlKind::Label {
                text: "Sales".into(),
            },
            rect: datacraft_core::canvas::Rect {
                x: 40.0,
                y: 40.0,
                width: 160.0,
                height: 24.0,
            },
            properties: datacraft_core::form::ControlProperties::default(),
        },
    });
    app.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Report,
        "Sheet",
    )));
    app
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            layout_probe::clear();
            datacraft_ui::report_design::report_preview_ui(app, &tab, ui);
        },
        app,
    );
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();
    harness
}

fn page_width(_harness: &Harness<'_, App>) -> f32 {
    layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "report.preview.page")
        .map(|(_, rect)| rect.width())
        .expect("the preview page records its rect")
}

#[test]
fn the_preview_zoom_scales_the_page_and_persists() {
    let dir = std::env::temp_dir().join(format!("datacraft-zoom-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");
    let mut harness = harness(report_app(path.clone()));

    // The default zoom (0.9) scales the A4 page.
    let default_width = page_width(&harness);
    assert!(
        (default_width - 595.0 * 0.9).abs() < 1.0,
        "default zoom: {default_width}"
    );

    harness.get_by_label("Zoom").click();
    harness.run();
    harness.get_by_label("150 %").click();
    harness.run();

    let zoomed = page_width(&harness);
    assert!(
        (zoomed - 595.0 * 1.5).abs() < 1.0,
        "the chosen zoom scales the page: {zoomed}"
    );
    let stored = std::fs::read_to_string(&path).expect("ui.json written");
    assert!(
        stored.contains("\"preview_zoom\": 1.5"),
        "the choice persists in ui.json: {stored}"
    );
}
