//! Filter by value in the datasheet (phase 101, step 101.2; item M132).
//!
//! The column header's context menu opens the value list; a click on an
//! entry filters the view through the bus, and the blank entry appears
//! only where the column carries a `Null` cell. Assertions stay on the
//! tab state and the rendered rows.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_core::state::ValueFilter;
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

/// Ada twice, Bob once and a blank name.
fn sample_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![FieldValue::Text("Ada".into()), FieldValue::Integer(1)],
        vec![FieldValue::Text("Bob".into()), FieldValue::Integer(2)],
        vec![FieldValue::Null, FieldValue::Integer(3)],
        vec![FieldValue::Text("Ada".into()), FieldValue::Integer(4)],
    ]
}

fn ready_app(rows: Vec<Vec<FieldValue>>) -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", rows);
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

fn open_value_list(harness: &mut Harness<'static, App>) {
    harness.run();
    harness.get_by_label("name").click_secondary();
    harness.run();
    harness.get_by_label("Filter by Value…").click();
    harness.run();
}

#[test]
fn the_header_menu_lists_the_values_and_filters_on_a_click() {
    let mut harness = datasheet_harness(ready_app(sample_rows()));
    open_value_list(&mut harness);
    // The window lists both names; the grid cells carry the same text, so
    // the window entries are the later nodes.
    let entries = harness.get_all_by_label("Ada").count();
    assert!(entries >= 2, "the list entry sits beside the cells: {entries}");
    harness
        .get_all_by_label("Ada")
        .last()
        .expect("the list entry")
        .click();
    harness.run();
    let tab = harness
        .state()
        .snapshot()
        .tabs
        .first()
        .cloned()
        .expect("a table tab");
    assert_eq!(
        tab.value_filters.get("name"),
        Some(&ValueFilter::Text("Ada".into()))
    );
}

#[test]
fn the_blank_entry_appears_only_for_a_column_with_nulls() {
    let mut harness = datasheet_harness(ready_app(sample_rows()));
    open_value_list(&mut harness);
    assert!(
        harness.query_by_label_contains("(Blanks)").is_some(),
        "the name column carries a Null cell"
    );
    // A null-free column lists no blank entry.
    let mut harness = datasheet_harness(ready_app(sample_rows()));
    harness.run();
    harness.get_by_label("qty").click_secondary();
    harness.run();
    harness.get_by_label("Filter by Value…").click();
    harness.run();
    assert!(harness.query_by_label_contains("(Blanks)").is_none());
}

#[test]
fn a_picked_value_dispatch_clears_the_previous_criteria() {
    let mut app = ready_app(sample_rows());
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "name".into(),
        criteria: Some("= \"Bob\"".into()),
    });
    let mut harness = datasheet_harness(app);
    open_value_list(&mut harness);
    harness
        .get_all_by_label("Ada")
        .last()
        .expect("the list entry")
        .click();
    harness.run();
    let tab = harness
        .state()
        .snapshot()
        .tabs
        .first()
        .cloned()
        .expect("a table tab");
    assert!(tab.filters.is_empty(), "the criteria made room");
    assert_eq!(
        tab.value_filters.get("name"),
        Some(&ValueFilter::Text("Ada".into()))
    );
}

#[test]
fn the_picked_value_hides_the_other_rows() {
    let mut harness = datasheet_harness(ready_app(sample_rows()));
    open_value_list(&mut harness);
    harness
        .get_all_by_label("Ada")
        .last()
        .expect("the list entry")
        .click();
    harness.run();
    assert!(
        harness.query_by_label_contains("Bob").is_none(),
        "the datasheet paints only the picked value"
    );
    // The window closes with the pick: no stray list remains.
    assert!(
        harness.query_by_label_contains("(Blanks)").is_none(),
        "the value list is gone"
    );
}
