//! View-state & affordance pins (steps 44.3–44.5: M52, M53, M54) —
//! split from `ui_kittest.rs` for the file-size gate, like the other
//! standalone kittest suites.
//!
//! Close Database on the backstage, the contextual Views split
//! switching the open table, and the datasheet's header-menu sort and
//! column filters. Data assertions on the semantic tree and the
//! snapshot — never pixels.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
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

fn other_table() -> TableDef {
    TableDef::new(
        "Others",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn customers_app(rows: Vec<Vec<datacraft_core::schema::FieldValue>>) -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", rows);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app
}

/// App with the Customers tab open (rows cached).
fn app_with_table_tab() -> App {
    let mut app = customers_app(vec![vec![
        datacraft_core::schema::FieldValue::Integer(1),
        datacraft_core::schema::FieldValue::Text("alpha".to_owned()),
    ]]);
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app
}

/// A two-row Customers app for the sort/filter pins (step 44.5, M54):
/// stored order beta, alpha — sorting by name must bring alpha first.
fn customers_two_rows_app() -> App {
    let mut app = customers_app(vec![
        vec![
            datacraft_core::schema::FieldValue::Integer(1),
            datacraft_core::schema::FieldValue::Text("beta".to_owned()),
        ],
        vec![
            datacraft_core::schema::FieldValue::Integer(2),
            datacraft_core::schema::FieldValue::Text("alpha".to_owned()),
        ],
    ]);
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app
}

fn themed(
    app: App,
    mut draw: impl FnMut(&mut eframe::egui::Ui, &mut App) + 'static,
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

fn backstage_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| {
        datacraft_ui::backstage::backstage_ui(app, ui)
    })
}

fn ribbon_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| datacraft_ui::ribbon::ribbon_ui(app, ui))
}

fn datasheet_harness(app: App) -> Harness<'static, App> {
    themed(app, |ui, app| {
        let Some(tab) = app.snapshot().tabs.first().cloned() else {
            return;
        };
        datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
    })
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump(harness: &Harness<'_, App>) -> String {
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

/// The backstage Save page closes the database over the bus (step 44.3,
/// item M52): exactly one `CloseDatabase`, back to the Home page, no
/// open source afterwards.
#[test]
fn the_backstage_close_database_button_closes_over_the_bus() {
    let mut app = customers_app(vec![vec![
        datacraft_core::schema::FieldValue::Integer(1),
        datacraft_core::schema::FieldValue::Text("alpha".to_owned()),
    ]]);
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label("Close Database").click();
    harness.run();
    let app = harness.state();
    assert!(
        app.snapshot().database.is_none(),
        "the database must be closed after the affordance dispatches"
    );
    assert_eq!(
        app.backstage_page,
        datacraft_ui::backstage::BackstagePage::Home,
        "the backstage lands on Home after closing"
    );
}

/// The contextual Table Tools Views split switches the OPEN tab's mode
/// in place (step 44.4, item M53): the same object stays active and one
/// `SetTabMode` moves it to the design view — while the Create tab's
/// Table Design keeps creating a NEW table (pinned in
/// ui_kittest_ribbon_truth.rs).
#[test]
fn the_contextual_views_button_switches_the_open_table_to_design() {
    let mut app = app_with_table_tab();
    app.active_ribbon_tab = "table_tools";
    let mut harness = ribbon_harness(app);
    harness.run();
    let object = harness.state().snapshot().tabs[0].object_name.clone();

    harness.get_by_label("Datasheet").click();
    harness.run();
    let tab = &harness.state().snapshot().tabs[0];
    assert!(
        matches!(tab.mode, datacraft_core::state::TabMode::Design),
        "the open table switches to the design view, got {:?}",
        tab.mode
    );
    assert_eq!(
        tab.object_name, object,
        "the same table stays open — the contextual switch never creates a new table"
    );
}

/// The datasheet header context menu sorts the TABLE (step 44.5, M54):
/// one menu action stores the core view state (sort field + direction
/// on the tab snapshot) and the rows render in the sorted order — alpha
/// (sorted) sits above beta, though the cache stores beta first.
#[test]
fn the_datasheet_header_menu_sorts_the_table() {
    let mut harness = datasheet_harness(customers_two_rows_app());
    harness.run();
    harness.get_by_label("name").click_secondary();
    harness.run();
    harness.get_by_label("Sort Ascending").click();
    harness.run();

    let tab = &harness.state().snapshot().tabs[0];
    assert_eq!(tab.sort_field.as_deref(), Some("name"));
    assert_eq!(
        tab.sort_order,
        Some(datacraft_core::qbe::SortOrder::Ascending)
    );
    let alpha = harness.get_by_label("alpha").rect();
    let beta = harness.get_by_label("beta").rect();
    assert!(
        alpha.top() < beta.top(),
        "the sorted row renders above the others (alpha {alpha:?}, beta {beta:?})"
    );
}

/// The header menu's Filter item opens the criteria dialog (step 45.1,
/// item M55 — the menu closed on editor click, so the editor lives in a
/// window now): typing a criterion and applying stores the filter over
/// the bus and the rows follow it.
#[test]
fn the_filter_dialog_applies_a_typed_criterion() {
    let mut harness = datasheet_harness(customers_two_rows_app());
    harness.run();
    harness.get_by_label("name").click_secondary();
    harness.run();
    harness.get_by_label("Filter…").click();
    harness.run();
    let editor =
        harness.get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Filter criteria");
    editor.set_text_value_accesskit("Like \"a*\"");
    harness.run();
    harness.get_by_label("Apply Filter").click();
    harness.run();

    let tab = &harness.state().snapshot().tabs[0];
    assert_eq!(
        tab.filters.get("name").map(String::as_str),
        Some("Like \"a*\""),
        "{:?}",
        tab.filters
    );
    assert!(harness.get_by_label("alpha").rect().width() > 0.0);
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("beta"),
        "the filtered-out row no longer renders:
{dump}"
    );
}

/// A set column filter hides the non-matching rows (step 44.5/45.1):
/// the dialog seeds its editor from the live filter, and the dialog's
/// Clear Filter removes the filter over the bus — beta returns.
#[test]
fn the_filter_dialog_seeds_and_clears_the_live_filter() {
    let mut app = customers_two_rows_app();
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "name".into(),
        criteria: Some("Like \"a*\"".into()),
    });
    let mut harness = datasheet_harness(app);
    harness.run();
    assert!(harness.get_by_label("alpha").rect().width() > 0.0);
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("beta"),
        "the filtered-out row no longer renders:
{dump}"
    );

    harness.get_by_label("name").click_secondary();
    harness.run();
    harness.get_by_label("Filter…").click();
    harness.run();
    let editor =
        harness.get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Filter criteria");
    assert_eq!(
        editor.value().as_deref(),
        Some("Like \"a*\""),
        "the active criteria are visible in the dialog"
    );

    harness.get_by_label("Clear Filter").click();
    harness.run();
    let tab = &harness.state().snapshot().tabs[0];
    assert!(tab.filters.is_empty(), "the filter cleared over the bus");
    assert!(
        harness.get_by_label("beta").rect().width() > 0.0,
        "beta returns"
    );
}

/// Activating an object surfaces its contextual tools tab (step 47.1,
/// item M58 — Access convention; the user had to click Tabellentools
/// manually): the ribbon's selection follows the active document's
/// kind.
#[test]
fn activating_a_table_surfaces_the_table_tools_tab() {
    let mut app = app_with_table_tab();
    app.active_ribbon_tab = "home";
    let mut harness = ribbon_harness(app);
    harness.run();
    assert_eq!(
        harness.state().active_ribbon_tab, "table_tools",
        "activating a table selects its contextual tools tab"
    );
}

/// The backstage is never yanked by object activation (step 47.1, M58
/// guard): File pages stay put.
#[test]
fn the_backstage_is_not_yanked_by_object_activation() {
    let mut app = app_with_table_tab();
    app.active_ribbon_tab = "file";
    let mut harness = ribbon_harness(app);
    harness.run();
    assert_eq!(
        harness.state().active_ribbon_tab, "file",
        "the backstage keeps the selection"
    );
}

/// Switching between two objects of the SAME kind never yanks the user
/// off a non-contextual ribbon tab (step 47.1, M58 guard): Database
/// Tools stays selected while the user browses tables.
#[test]
fn switching_between_two_tables_keeps_the_ribbon_tab() {
    let port = MockPort::with_tables(vec![customers(), other_table()]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Others",
    )));
    app.active_ribbon_tab = "database_tools";
    let first = app.snapshot().tabs[0].id;
    app.dispatch(Command::SetActiveTab { tab_id: first });
    let mut harness = ribbon_harness(app);
    harness.run();
    assert_eq!(
        harness.state().active_ribbon_tab, "database_tools",
        "same-kind activation keeps the non-contextual tab"
    );
}
