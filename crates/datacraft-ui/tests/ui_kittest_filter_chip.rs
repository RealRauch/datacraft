//! Status filter-chip suite (step 69.5, item M83).
//!
//! The chip keeps the count in its label, lists every active filter with a
//! per-entry remove action and keeps the clear-all behavior.

use std::collections::BTreeMap;

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
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

fn app_with_filters() -> App {
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
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "name".into(),
        criteria: Some("a*".into()),
    });
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "id".into(),
        criteria: Some(">1".into()),
    });
    assert_eq!(filters(&app).len(), 2);
    app
}

fn filters(app: &App) -> BTreeMap<String, String> {
    app.snapshot()
        .tabs
        .iter()
        .find(|tab| tab.object_name == "Customers")
        .map(|tab| tab.filters.clone())
        .unwrap_or_default()
}

fn chip_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let filters = filters(app);
            if let Some(command) =
                datacraft_ui::shell::filter_chip_ui(ui, "Customers", &filters)
            {
                let _ = app.dispatch(command);
            }
        },
        app,
    )
}

/// Removing one entry leaves the other filter in place.
#[test]
fn the_chip_lists_and_removes_individual_filters() {
    let mut harness = chip_harness(app_with_filters());
    harness.run();
    harness.get_by_label("Filter (2)").click();
    harness.run();

    harness.get_by_label("name: a*").click();
    harness.run();

    let remaining = filters(harness.state());
    assert_eq!(remaining.len(), 1, "one filter was removed: {remaining:?}");
    assert!(remaining.contains_key("id"), "the other filter remains");
}

/// The clear-all entry keeps the old one-click behavior.
#[test]
fn the_chip_clears_all_filters() {
    let mut harness = chip_harness(app_with_filters());
    harness.run();
    harness.get_by_label("Filter (2)").click();
    harness.run();

    harness.get_by_label("Clear all filters").click();
    harness.run();

    assert!(filters(harness.state()).is_empty());
}
