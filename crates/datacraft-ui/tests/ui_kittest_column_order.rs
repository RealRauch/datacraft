//! Column order and the Null marker (phase 101, step 101.4; item M132).
//!
//! The header menu moves a column through the display order (the keyboard
//! and agent twin of the drag gesture); the grid then paints the headers,
//! widths and cells in that order while the storage columns stay put. A
//! `Null` cell carries a visible marker instead of an empty cell.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn ready_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Text("Ada".into()), FieldValue::Integer(1)],
            vec![FieldValue::Null, FieldValue::Integer(2)],
        ],
    );
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
    app
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
fn a_null_cell_carries_the_visible_marker() {
    let harness = datasheet_harness(ready_app());
    assert!(
        harness.query_by_label("(NULL)").is_some(),
        "the Null cell is distinct from an empty cell"
    );
}

#[test]
fn the_header_menu_moves_a_column_right() {
    let mut harness = datasheet_harness(ready_app());
    harness.run();
    let name_before = harness.get_by_label("name").rect().left();
    let qty_before = harness.get_by_label("qty").rect().left();
    assert!(name_before < qty_before, "storage order first");
    harness.get_by_label("name").click_secondary();
    harness.run();
    harness.get_by_label("Move Column Right").click();
    harness.run();
    let name_after = harness.get_by_label("name").rect().left();
    let qty_after = harness.get_by_label("qty").rect().left();
    assert!(
        qty_after < name_after,
        "the display order swapped: qty {qty_after}, name {name_after}"
    );
    // The cells moved with their column: row 0's leftmost cell is qty's
    // value, not the name.
    let one = harness.get_by_label("1").rect().left();
    let ada = harness.get_by_label("Ada").rect().left();
    assert!(one < ada, "the values follow their headers: {one} / {ada}");
}

#[test]
fn a_moved_column_keeps_its_sort_by_name() {
    let mut harness = datasheet_harness(ready_app());
    harness.run();
    harness.get_by_label("qty").click_secondary();
    harness.run();
    harness.get_by_label("Sort Descending").click();
    harness.run();
    harness.get_by_label("qty").click_secondary();
    harness.run();
    harness.get_by_label("Move Column Left").click();
    harness.run();
    let tab = &harness.state().snapshot().tabs[0];
    assert_eq!(
        tab.sort_field.as_deref(),
        Some("qty"),
        "the sort addresses the column, not its position"
    );
}

#[test]
fn a_cell_edit_lands_on_the_storage_column_after_a_move() {
    let mut harness = datasheet_harness(ready_app());
    harness.run();
    // Move qty to the left, then edit the leftmost value of row 0.
    harness.get_by_label("qty").click_secondary();
    harness.run();
    harness.get_by_label("Move Column Left").click();
    harness.run();
    let tab_id = harness.state().snapshot().tabs[0].id;
    harness
        .state_mut()
        .arm_cell_edit(tab_id, 0, 0, "7".to_owned());
    assert!(
        harness.state_mut().cell_edit_parts(tab_id).is_some(),
        "the editor is armed"
    );
    harness
        .state_mut()
        .commit_cell_edit(tab_id, "Customers", 0, 0);
    harness.run();
    // The driver writes immediately (no atomic batches): the value lands
    // through one UpdateCells dispatch and the grid repaints it, left of
    // the name — the moved column kept its identity.
    let seven = harness.get_by_label("7").rect().left();
    let ada = harness.get_by_label("Ada").rect().left();
    assert!(seven < ada, "qty renders left of the name: {seven} / {ada}");
}
