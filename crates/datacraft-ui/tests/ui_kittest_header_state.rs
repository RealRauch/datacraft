//! Datasheet header-state suite (step 69.6, item M84).
//!
//! A filtered or sorted column names its state in the header's hover text;
//! the indicator glyphs are painted, so the semantic text is the pin.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::SortOrder;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn app_with_table() -> App {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![customers()])));
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

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut text = String::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        if label.is_empty() {
            text.push_str(&node.value().unwrap_or_default());
        } else {
            text.push_str(&label);
        }
        text.push('|');
    }
    text
}

fn hover_id_header(harness: &mut Harness<'static, App>) {
    let header = harness
        .get_all_by_label("Id")
        .find(|node| format!("{:?}", node.accesskit_node().role()) == "ColumnHeader")
        .expect("the Id column header");
    let center = header.rect().center();
    harness.hover_at(center);
    harness.run();
}

/// A filtered column names its criteria on hover.
#[test]
fn a_filtered_header_names_the_criteria() {
    let mut app = app_with_table();
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "Id".into(),
        criteria: Some("> 1".into()),
    });
    let mut harness = datasheet_harness(app);
    harness.run();
    hover_id_header(&mut harness);

    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Filter: > 1"),
        "the filtered header names the criteria: {dump}"
    );
}

/// A sorted column names its direction on hover.
#[test]
fn a_sorted_header_names_the_direction() {
    let mut app = app_with_table();
    app.dispatch(Command::SetTableSort {
        table: "Customers".into(),
        field: "Id".into(),
        order: SortOrder::Ascending,
    });
    let mut harness = datasheet_harness(app);
    harness.run();
    hover_id_header(&mut harness);

    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Sorted ascending"),
        "the sorted header names the direction: {dump}"
    );
}
