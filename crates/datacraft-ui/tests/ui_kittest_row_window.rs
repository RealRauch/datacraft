//! Windowed datasheet rows (phase 101, step 101.5; item M132).
//!
//! A table above the threshold opens without reading every row: the port's
//! count sizes the list, one window is loaded, and scrolling loads the
//! next. "Load all rows" is the explicit upgrade; a filter, a sort or a
//! staged batch upgrades on its own.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// A table far above the windowing threshold.
fn large_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                FieldValue::Integer(index as i64),
                FieldValue::Text(format!("name{index:05}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

fn datasheet_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

#[test]
fn a_large_table_opens_with_one_window_and_no_full_read() {
    let (app, tab) = large_app(2500);
    let mut harness = datasheet_harness(app);
    harness.set_size(egui::vec2(700.0, 500.0));
    harness.run();
    assert!(
        harness.state().rows_for(tab).is_none(),
        "the full cache stays empty"
    );
    assert_eq!(harness.state().loaded_row_windows(tab), Some(1));
    assert!(
        harness
            .query_by_label_contains("of 2500 rows are loaded in windows")
            .is_some(),
        "the footer states the windowed state"
    );
    assert!(
        harness.query_by_label("name00000").is_some(),
        "the loaded window paints"
    );
}

#[test]
fn scrolling_loads_the_next_window() {
    let (app, tab) = large_app(2500);
    let mut harness = datasheet_harness(app);
    harness.set_size(egui::vec2(700.0, 500.0));
    harness.run();
    harness.hover_at(egui::pos2(300.0, 200.0));
    harness.run();
    // One window covers WINDOW_ROWS rows; scroll past it to need the next.
    for _ in 0..40 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -220.0),
            modifiers: egui::Modifiers::default(),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        harness.run();
    }
    let loaded = harness.state().loaded_row_windows(tab).expect("windowed");
    assert!(loaded >= 2, "scrolling pulled another window in: {loaded}");
    assert!(
        harness.state().rows_for(tab).is_none(),
        "still no full read"
    );
}

#[test]
fn load_all_rows_is_the_explicit_upgrade() {
    let (app, tab) = large_app(2500);
    let mut harness = datasheet_harness(app);
    harness.set_size(egui::vec2(700.0, 500.0));
    harness.run();
    harness.get_by_label("Load all rows").click();
    harness.run();
    assert_eq!(
        harness.state().rows_for(tab).map(Vec::len),
        Some(2500),
        "the whole table arrived"
    );
    assert_eq!(harness.state().loaded_row_windows(tab), None);
    assert!(
        harness
            .query_by_label_contains("are loaded in windows")
            .is_none(),
        "the footer is gone"
    );
}

#[test]
fn a_sort_upgrades_the_tab_automatically() {
    let (app, tab) = large_app(2500);
    let mut harness = datasheet_harness(app);
    harness.set_size(egui::vec2(700.0, 500.0));
    harness.run();
    // A sort can arrive from any client (the wire never sees the cache).
    harness.state_mut().dispatch(Command::SetTableSort {
        table: "Customers".into(),
        field: "name".into(),
        order: datacraft_core::qbe::SortOrder::Descending,
    });
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().rows_for(tab).map(Vec::len),
        Some(2500),
        "the sort needed every row and got them"
    );
    assert_eq!(harness.state().loaded_row_windows(tab), None);
}
