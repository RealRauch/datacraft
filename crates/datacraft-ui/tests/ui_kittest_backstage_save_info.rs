//! The backstage Save/Info contracts (step 91.8, items M116/L59): every
//! Save-page section carries a consistent heading, and the Info page's
//! fact values align into one shared column (the value rects ride the
//! layout probe under `backstage.info.fact_value`).

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn app_with_database() -> App {
    let mut port = MockPort::with_tables(vec![datacraft_core::schema::TableDef::new(
        "Customers",
        vec![datacraft_core::schema::FieldDef::new(
            "Id",
            datacraft_core::schema::FieldKind::Number(
                datacraft_core::schema::NumericKind::LongInteger,
            ),
        )],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![datacraft_core::schema::FieldValue::Integer(1)]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn backstage_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    )
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        lines.push(format!("{:?}|{label}|{value}", node.role()));
    }
    lines.join("\n")
}

/// The Save page's three sections all carry headings (M116): the sidecar
/// block no longer starts with a bare field label.
#[test]
fn the_save_page_names_every_section() {
    let mut app = app_with_database();
    app.backstage_page = BackstagePage::Save;
    app.active_ribbon_tab = "file";
    let mut harness = backstage_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Save a Copy") && dump.contains("Database Password"),
        "the two headed sections are present:\n{dump}"
    );
    assert!(
        dump.contains("Design Sidecar"),
        "the sidecar section carries its heading:\n{dump}"
    );
}

/// The Info page's fact values share one left edge (L59) — the ragged
/// label widths no longer decide where the values start.
#[test]
fn the_info_page_aligns_fact_values_into_one_column() {
    let mut app = app_with_database();
    app.backstage_page = BackstagePage::Info;
    app.active_ribbon_tab = "file";
    let mut harness = backstage_harness(app);
    harness.run();
    let rects: Vec<egui::Rect> = layout_probe::drain()
        .iter()
        .filter(|(id, _)| id == "backstage.info.fact_value")
        .map(|(_, rect)| *rect)
        .collect();
    assert!(
        rects.len() >= 3,
        "the fact rows record their value rects: {rects:?}"
    );
    let first = rects[0].left();
    assert!(
        rects.iter().all(|rect| (rect.left() - first).abs() < 0.5),
        "every value starts at the same column edge: {rects:?}"
    );
}
