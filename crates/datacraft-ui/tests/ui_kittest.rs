//! Headless UI suite (step 23.2, item T5).
//!
//! `egui_kittest` drives the real widgets through AccessKit: interactions
//! select labels/roles instead of pixels, and the semantic tree dump is the
//! layout gate (committed `insta` snapshots). Assertions stay on domain
//! state (`UiStateSnapshot`, cached results) — never on rendered pixels.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App over a mock database with one table and one row.
fn mock_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![vec![
            datacraft_core::schema::FieldValue::Integer(1),
            datacraft_core::schema::FieldValue::Text("alpha".to_owned()),
        ]],
    );
    let mut app = App::new(Box::new(port));
    let events = app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    app
}

/// App with the Customers tab open (rows cached).
fn app_with_table_tab() -> (App, TabId) {
    let mut app = mock_app();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

fn themed(
    app: App,
    mut draw: impl FnMut(&mut egui::Ui, &mut App) + 'static,
) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            draw(ui, app);
        },
        app,
    )
}

fn ribbon_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| datacraft_ui::ribbon::ribbon_ui(app, ui))
}

fn nav_pane_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| datacraft_ui::nav_pane::nav_pane_ui(app, ui))
}

fn datasheet_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| {
        let Some(tab) = app.snapshot().tabs.first().cloned() else {
            return;
        };
        datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
    })
}

fn backstage_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| {
        datacraft_ui::backstage::backstage_ui(app, ui)
    })
}

#[test]
fn ribbon_click_toggles_the_nav_pane_in_the_domain_snapshot() {
    let mut harness = ribbon_harness(mock_app());
    assert!(harness.state().snapshot().nav_pane_visible);
    harness.get_by_label("Navigation Pane").click();
    harness.run();
    assert!(
        !harness.state().snapshot().nav_pane_visible,
        "the ribbon action must reach the command bus"
    );
}

#[test]
fn nav_pane_click_opens_the_table_tab() {
    let mut harness = nav_pane_harness(mock_app());
    harness.get_by_label("Customers").click();
    harness.run();
    let snapshot = harness.state().snapshot();
    assert_eq!(snapshot.tabs.len(), 1);
    assert_eq!(snapshot.tabs[0].object_name, "Customers");
}

#[test]
fn datasheet_exposes_its_cells_as_labels() {
    let (app, _tab) = app_with_table_tab();
    let harness = datasheet_harness(app);
    let cell = harness.get_by_label("alpha").rect();
    assert!(cell.width() > 0.0, "the cell has a painted rect");
    let count = harness.get_by_label_contains("row").rect();
    assert!(count.width() > 0.0, "the row-count label renders");
}

#[test]
fn sql_window_runs_the_default_statement_through_the_bus() {
    let mut app = mock_app();
    app.dispatch(Command::ToggleSqlWindow);
    let mut harness = themed(app, |ui, app| datacraft_ui::sql_window::sql_window(app, ui));
    assert!(harness.state().snapshot().sql_window_open);
    harness.get_by_label("Run").click();
    harness.run();
    let result = harness
        .state()
        .sql_result()
        .expect("the default SELECT 1 statement runs");
    assert_eq!(result.rows.len(), 1);
}

#[test]
fn backstage_home_navigates_to_the_open_page() {
    let mut harness = backstage_harness(mock_app());
    harness.get_by_label("Open Database").click();
    harness.run();
    assert_eq!(
        harness.state().backstage_page,
        datacraft_ui::backstage::BackstagePage::Open
    );
}

#[test]
fn ribbon_tree_matches_the_layout_snapshot() {
    let harness = ribbon_harness(mock_app());
    insta::assert_snapshot!("ribbon_home", tree_dump(&harness));
}

#[test]
fn nav_pane_tree_matches_the_layout_snapshot() {
    let harness = nav_pane_harness(mock_app());
    insta::assert_snapshot!("nav_pane", tree_dump(&harness));
}

#[test]
fn datasheet_tree_matches_the_layout_snapshot() {
    let (app, _tab) = app_with_table_tab();
    let harness = datasheet_harness(app);
    insta::assert_snapshot!("datasheet", tree_dump(&harness));
}

#[test]
fn sql_window_tree_matches_the_layout_snapshot() {
    let mut app = mock_app();
    app.dispatch(Command::ToggleSqlWindow);
    let harness = themed(app, |ui, app| datacraft_ui::sql_window::sql_window(app, ui));
    insta::assert_snapshot!("sql_window", tree_dump(&harness));
}

#[test]
fn backstage_tree_matches_the_layout_snapshot() {
    let harness = backstage_harness(mock_app());
    insta::assert_snapshot!("backstage_home", tree_dump(&harness));
}
