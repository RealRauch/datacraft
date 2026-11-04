//! Keyboard record navigation suite (step 40.1, item M38).
//!
//! The datasheet grid answers the navigation keys with the record cursor:
//! a row-selector click arms the grid (egui focus, kept by a lock filter
//! against the arrow-key focus walk), hovering arms it as long as no text
//! field holds focus. Assertions stay on the presentation cursor; the
//! scroll-follow math (`ensure_row_visible`, `page_rows`) is pinned by the
//! core contract suite (`grid_core.rs`).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
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

/// App with the Customers tab open over `rows` records. Names carry their
/// index so the headless suite can address cells by label.
fn multi_row_table_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
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

/// Clicking a row selector selects that record and arms the grid; the
/// navigation keys then walk the cursor — clamped at both ends, with the
/// home/end jumps Access datasheets provide.
#[test]
fn navigation_keys_move_the_record_cursor_from_the_row_selector() {
    let (app, tab) = multi_row_table_app(6);
    let mut harness = datasheet_harness(app);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 0);

    // Clicking the third record's selector selects it and anchors the grid.
    harness.get_by_label("     3").click();
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 2);

    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 3);

    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 2);

    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 0);

    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 5);

    // One row past the end clamps to the last record.
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().record_cursor(tab), 5);
}

/// Hovering the grid also arms the navigation keys, and PageDown jumps by
/// the viewport. No click happened before, so this pins the pointer gate.
#[test]
fn hovering_the_grid_arms_page_and_arrow_navigation() {
    let (app, tab) = multi_row_table_app(30);
    let mut harness = datasheet_harness(app);
    harness.run();

    let cell = harness.get_by_label("alpha4").rect();
    harness.hover_at(cell.center());
    harness.run();

    harness.key_press(egui::Key::PageDown);
    harness.run();
    let after_page = harness.state().record_cursor(tab);
    assert!(after_page > 4, "PageDown jumped forward, got {after_page}");

    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(
        harness.state().record_cursor(tab),
        after_page - 1,
        "ArrowUp walks back exactly one record"
    );
}
