//! Scratch: bisect the report-design combo popup failure.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn setup() -> (App, datacraft_core::snapshot::TabSnapshot) {
    let port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("total", FieldKind::Currency),
        ],
    )]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::CreateReport {
        name: "Roster".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Report, "Roster", "Orders");
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Report,
        "Roster",
    )));
    let tab = app.snapshot().tabs[0].clone();
    app.dispatch(Command::SetTabMode {
        tab_id: tab.id,
        mode: datacraft_core::state::TabMode::Design,
    });
    (app, tab)
}

fn canvas_scratch(ui: &mut egui::Ui, report: &datacraft_core::report::ReportDef) {
    let page = egui::vec2(report.page.width, report.page.height);
    let (canvas_rect, _) = ui.allocate_exact_size(page, egui::Sense::hover());
    println!("CANVAS: {canvas_rect:?}");
    let painter = ui.painter_at(canvas_rect);
    painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
}

fn only_picker(app: &mut App, ui: &mut egui::Ui) {
    let report = app.snapshot().database.as_ref().expect("open").reports[0].clone();
    let fields: Vec<String> = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| {
            db.tables
                .iter()
                .find(|table| table.name.eq_ignore_ascii_case(&report.source))
                .map(|table| table.fields.iter().map(|f| f.name.clone()).collect())
        })
        .unwrap_or_default();
    let selected = report
        .group_by
        .clone()
        .unwrap_or_else(|| "no grouping".to_owned());
    egui::ComboBox::from_label("Grouping")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            let _ = ui.selectable_label(report.group_by.is_none(), "no grouping");
            for field in &fields {
                let _ = ui.selectable_label(false, field);
            }
        });
}

/// horizontal_top + canvas + picker (no palette).
#[test]
fn scratch_horizontal_canvas_picker() {
    let (app, tab) = setup();
    let mut harness = Harness::new_ui_state(
        move |ui, (app, tab)| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let report = app.snapshot().database.as_ref().expect("open").reports[0].clone();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| canvas_scratch(ui, &report));
                ui.separator();
                ui.vertical(|ui| {
                    datacraft_ui::designer_palette::designer_palette_ui(
                        app,
                        datacraft_core::ObjectKind::Report,
                        "Roster",
                        ui,
                    );
                    only_picker(app, ui);
                });
            });
            let _ = tab;
        },
        (app, tab),
    );
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();
    println!("COMBO: {:?}", harness.get_by_label("Grouping").rect());
    harness.get_by_label("Grouping").click();
    harness.run();
    harness.get_by_label("customer_id");
    println!("SCRATCH_HCP_OK");
}
