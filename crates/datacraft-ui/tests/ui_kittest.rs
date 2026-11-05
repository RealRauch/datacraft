//! Headless UI suite (step 23.2, item T5).
//!
//! `egui_kittest` drives the real widgets through AccessKit: interactions
//! select labels/roles instead of pixels, and the semantic tree dump is the
//! layout gate (committed `insta` snapshots). Assertions stay on domain
//! state (`UiStateSnapshot`, cached results) — never on rendered pixels.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::{
    DriverCapabilities, MockPort, NativeQuery, NativeQueryColumn, NativeQueryKind,
};
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

/// The New Database page (phase 29, H44): selecting the blank template and
/// clicking Create dispatches create + open — the new database is open.
#[test]
fn backstage_new_page_creates_from_a_template() {
    let mut app = mock_app();
    app.dispatch(Command::CloseDatabase);
    let path =
        std::env::temp_dir().join(format!("datacraft-kittest-{}-new.mdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    app.new_database_path = path.to_string_lossy().into_owned();
    let mut harness = backstage_harness(app);

    harness.get_by_label("New").click();
    harness.run();
    harness.get_by_label("Blank database — 0 tables").click();
    harness.run();
    harness.get_by_label("Create Database").click();
    harness.run();

    let database = harness
        .state()
        .snapshot()
        .database
        .as_ref()
        .expect("the created database opens");
    assert_eq!(database.format.to_string(), "Jet4");
    let _ = std::fs::remove_file(&path);
}

/// The Samples page (phase 29, H44): the dev-workspace samples resolve and
/// one click opens the shipped database.
#[test]
fn backstage_samples_page_lists_and_opens_shipped_samples() {
    let mut app = mock_app();
    app.dispatch(Command::CloseDatabase);
    let mut harness = backstage_harness(app);

    harness.get_by_label("Samples").click();
    harness.run();
    harness.get_by_label("blank.mdb").click();
    harness.run();

    let database = harness
        .state()
        .snapshot()
        .database
        .as_ref()
        .expect("the sample opens");
    assert_eq!(database.format.to_string(), "Jet4");
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

/// Capability-restricted sources disable the matching affordances
/// (phase 30, item M17; ADR-7): the ribbon Table action and the
/// datasheet's new-record row render disabled with the reason, while a
/// capable source (the default mock) leaves everything enabled. Data
/// assertions on the semantic tree — never pixels.
#[test]
fn capability_restricted_sources_disable_the_matching_affordances() {
    let mut caps = datacraft_core::port::DriverCapabilities::full();
    caps.create_table = false;
    caps.write_rows = false;
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port));
    let events = app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    assert!(events.len() == 1, "{events:?}");
    let (mut app, _tab) = {
        app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            datacraft_core::ObjectKind::Table,
            "Customers",
        )));
        let tab = app.snapshot().tabs[0].id;
        (app, tab)
    };
    // The Table action lives on the Create tab.
    app.active_ribbon_tab = "create";

    let mut ribbon = ribbon_harness(app);
    ribbon.run();
    let dump = tree_dump(&ribbon);
    assert!(
        dump.contains("Button|Table| disabled"),
        "the Table action renders disabled:\n{dump}"
    );

    let (app, _tab) = {
        let mut port = MockPort::with_tables(vec![customers()]);
        port.with_capabilities(caps);
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
    };
    let mut datasheet = datasheet_harness(app);
    datasheet.run();
    let dump = tree_dump(&datasheet);
    assert!(
        dump.contains("Button|Add record| disabled"),
        "the new-record row renders disabled:\n{dump}"
    );

    // Contrast: a capable source keeps both affordances interactive.
    let mut ribbon = ribbon_harness(mock_app());
    ribbon.run();
    let dump = tree_dump(&ribbon);
    assert!(
        !dump.contains("Button|Table| disabled"),
        "a capable source keeps the Table action enabled:\n{dump}"
    );
}

#[test]
fn native_query_run_executes_through_the_ui() {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![vec![
            datacraft_core::schema::FieldValue::Integer(1),
            datacraft_core::schema::FieldValue::Text("alpha".to_owned()),
        ]],
    );
    port.with_native_queries(vec![NativeQuery {
        name: "CustomerList".to_owned(),
        kind: NativeQueryKind::Select,
        tables: vec!["Customers".to_owned()],
        columns: vec![
            NativeQueryColumn {
                table: Some("Customers".to_owned()),
                field: Some("name".to_owned()),
                expression: None,
                descending: false,
            },
            NativeQueryColumn {
                table: Some("Customers".to_owned()),
                field: Some("id".to_owned()),
                expression: None,
                descending: false,
            },
        ],
        criteria: vec!["Customers.id > 0".to_owned()],
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Query,
        "CustomerList",
    )));
    let mut harness = themed(app, |ui, app| {
        let Some(tab) = app.snapshot().tabs.first().cloned() else {
            return;
        };
        datacraft_ui::qbe::qbe_ui(app, &tab, ui);
    });
    harness.run();
    harness.get_by_label("Run").click();
    harness.step();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Ran — 1 row"),
        "the native query executed through the bus:\n{dump}"
    );
}

/// A mock source that honours the password surface (the default mock
/// reports `passwords: false`).
fn password_capable_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(DriverCapabilities::full());
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app
}

#[test]
fn backstage_save_copy_dispatches_one_command() {
    let mut app = password_capable_app();
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    app.save_as_path = "copy.mdb".to_owned();
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label("Save Copy").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(dump.contains("Copy saved to copy.mdb"), "{dump}");
}

#[test]
fn backstage_change_password_dispatches_the_command() {
    let mut app = password_capable_app();
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    app.change_password = "secret".to_owned();
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label("Change Password").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(dump.contains("Database password changed"), "{dump}");
}

#[test]
fn backstage_password_controls_gate_on_the_capability() {
    let mut app = mock_app();
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    let mut harness = backstage_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("the open source has no database passwords"),
        "{dump}"
    );
    assert!(dump.contains("Button|Save Copy| disabled"), "{dump}");
    assert!(dump.contains("Button|Change Password| disabled"), "{dump}");
}

/// The backstage Save page closes the database over the bus (step 44.3,
/// item M52): exactly one `CloseDatabase`, back to the Home page, no
/// open source afterwards.
#[test]
fn the_backstage_close_database_button_closes_over_the_bus() {
    let mut app = mock_app();
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

#[test]
fn qat_undo_and_redo_follow_the_history_depth() {
    let mut harness = ribbon_harness(mock_app());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(dump.contains("Button|Undo| disabled"), "{dump}");
    assert!(dump.contains("Button|Redo| disabled"), "{dump}");
    drop(harness);

    let mut app = mock_app();
    app.dispatch(Command::InsertRow {
        table: "Customers".to_owned(),
        values: vec![
            datacraft_core::schema::FieldValue::Integer(2),
            datacraft_core::schema::FieldValue::Text("beta".to_owned()),
        ],
    });
    let mut harness = ribbon_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(!dump.contains("Button|Undo| disabled"), "{dump}");
    assert_eq!(harness.state().snapshot().undo_depth, 1);

    harness.get_by_label("Undo").click();
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);
    assert_eq!(harness.state().snapshot().redo_depth, 1);
    let dump = tree_dump(&harness);
    assert!(dump.contains("Button|Undo| disabled"), "{dump}");
    assert!(!dump.contains("Button|Redo| disabled"), "{dump}");
}
