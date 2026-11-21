//! Docked task-pane host suite (step 92.2, item M108).
//!
//! Pins the dock/float switch through the pane header's own menu, the
//! no-overlap contract of the docked panel and the mode's restart
//! survival through `ui.json`.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use datacraft_ui::panes::{DockMode, PaneGeometry};
use datacraft_ui::settings::Settings;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn sql_app() -> App {
    let port = MockPort::with_tables(vec![datacraft_core::schema::TableDef::new(
        "Orders",
        vec![datacraft_core::schema::FieldDef::new(
            "Name",
            datacraft_core::schema::FieldKind::Text { max_len: 40 },
        )],
    )]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    app.dispatch(Command::ToggleSqlWindow);
    app
}

/// The full shell at the shipped default size; every frame clears and the
/// caller drains the rect log.
fn pane_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            // The language static is process-wide; parallel tests in this
            // binary would race the host locale otherwise.
            datacraft_ui::i18n::set_lang(datacraft_ui::i18n::Lang::English);
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

fn rect_of<'a>(rects: &'a [(String, egui::Rect)], id: &str) -> &'a egui::Rect {
    rects
        .iter()
        .find(|(name, _)| name == id)
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| panic!("rect {id:?} recorded"))
}

fn has_id(rects: &[(String, egui::Rect)], id: &str) -> bool {
    rects.iter().any(|(name, _)| name == id)
}

/// The header menu docks the SQL pane right, back to float, and the
/// docked size lands in the settings capture (step 92.2, item M108).
#[test]
fn the_header_menu_docks_the_sql_pane_right_and_back_to_float() {
    let mut harness = pane_harness(sql_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "sql.window") && !has_id(&rects, "panes.sql_panel"),
        "the pane starts floating"
    );

    harness.get_by_label("Dock").click();
    harness.run();
    harness.get_by_label("Dock Right").click();
    harness.run();
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "panes.sql_panel") && !has_id(&rects, "sql.window"),
        "docking right replaces the floating window"
    );
    let pane = rect_of(&rects, "panes.sql_panel");
    let workspace = rect_of(&rects, "shell.workspace_panel");
    assert!(
        workspace.right() <= pane.left() + 1.0,
        "the workspace overlaps the docked pane: {workspace:?} vs {pane:?}"
    );
    let size = harness.state().pane_geometry("sql").size;
    assert!(
        size >= PaneGeometry::RIGHT_MIN,
        "the dock width is captured: {size}"
    );

    harness.get_by_label("Dock").click();
    harness.run();
    harness.get_by_label("Float").click();
    harness.run();
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "sql.window") && !has_id(&rects, "panes.sql_panel"),
        "floating restores the window"
    );
}

/// An app with two related tables and the relationships mapper open
/// (step 93.1, item M125).
fn relationships_app() -> App {
    let mut port = MockPort::with_tables(vec![
        datacraft_core::schema::TableDef::new(
            "Customers",
            vec![datacraft_core::schema::FieldDef::new(
                "id",
                datacraft_core::schema::FieldKind::Number(
                    datacraft_core::schema::NumericKind::LongInteger,
                ),
            )],
        ),
        datacraft_core::schema::TableDef::new(
            "Orders",
            vec![datacraft_core::schema::FieldDef::new(
                "customer_id",
                datacraft_core::schema::FieldKind::Number(
                    datacraft_core::schema::NumericKind::LongInteger,
                ),
            )],
        ),
    ]);
    port.with_relationships(vec![datacraft_core::port::Relationship {
        table: "Customers".into(),
        column: "id".into(),
        related_table: "Orders".into(),
        related_column: "customer_id".into(),
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleRelationshipMapper);
    app
}

/// The header menu docks the relationships pane right, back to float, and
/// the docked size lands in the settings capture (step 93.1, item M125).
#[test]
fn the_header_menu_docks_the_relationships_pane_right_and_back_to_float() {
    let mut harness = pane_harness(relationships_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "relationships.window") && !has_id(&rects, "panes.relationships_panel"),
        "the pane starts floating"
    );

    harness.get_by_label("Dock").click();
    harness.run();
    harness.get_by_label("Dock Right").click();
    harness.run();
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "panes.relationships_panel") && !has_id(&rects, "relationships.window"),
        "docking right replaces the floating window"
    );
    let pane = rect_of(&rects, "panes.relationships_panel");
    let workspace = rect_of(&rects, "shell.workspace_panel");
    assert!(
        workspace.right() <= pane.left() + 1.0,
        "the workspace overlaps the docked pane: {workspace:?} vs {pane:?}"
    );
    let size = harness.state().pane_geometry("relationships").size;
    assert!(
        size >= PaneGeometry::RIGHT_MIN,
        "the dock width is captured: {size}"
    );

    harness.get_by_label("Dock").click();
    harness.run();
    harness.get_by_label("Float").click();
    harness.run();
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "relationships.window") && !has_id(&rects, "panes.relationships_panel"),
        "floating restores the window"
    );
}

/// The relationships pane's dock mode survives a restart: the settings
/// file restores it and the restarted shell renders the pane docked
/// without interaction (step 93.1, item M125).
#[test]
fn the_relationships_pane_mode_survives_a_restart_from_the_settings_file() {
    let dir = std::env::temp_dir().join(format!(
        "datacraft-panes-relationships-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    let mut app = relationships_app();
    app.set_settings_path(Some(path.clone()));
    app.set_pane_mode("relationships", DockMode::Right);
    app.persist_settings().expect("save");

    let mut loaded = Settings::load(&path);
    assert_eq!(loaded.panes["relationships"].mode, DockMode::Right);
    // Pin the language: Auto would resolve the host locale process-wide.
    loaded.language = datacraft_ui::settings::LanguageSetting::English;

    let mut restarted = relationships_app();
    restarted.apply_settings(loaded);
    let mut harness = pane_harness(restarted);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "panes.relationships_panel"),
        "the restarted shell renders the pane docked"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The SQL pane's restart pin (step 92.2, item M108) — kept next to the
/// relationships one so both panes stay covered by the same suite.
#[test]
fn the_sql_pane_mode_survives_a_restart_from_the_settings_file() {
    let dir = std::env::temp_dir().join(format!("datacraft-panes-sql-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    let mut app = sql_app();
    app.set_settings_path(Some(path.clone()));
    app.set_pane_mode("sql", DockMode::Right);
    app.persist_settings().expect("save");

    let mut loaded = Settings::load(&path);
    assert_eq!(loaded.panes["sql"].mode, DockMode::Right);
    // Pin the language: Auto would resolve the host locale process-wide.
    loaded.language = datacraft_ui::settings::LanguageSetting::English;

    let mut restarted = sql_app();
    restarted.apply_settings(loaded);
    let mut harness = pane_harness(restarted);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    let rects = layout_probe::drain();
    assert!(
        has_id(&rects, "panes.sql_panel"),
        "the restarted shell renders the pane docked"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
