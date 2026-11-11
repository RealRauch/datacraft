//! Backstage flow suite over the busy seam (step 68.5, item M75; moved
//! out of `ui_kittest.rs` — L30).
//!
//! These flows dispatch whole-file commands, which now run one painted
//! frame later through `busy::busy_frame`; the harness therefore renders
//! the real seam.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
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
        source: None,
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    app
}

/// App with the database open and every capability (password flows).
fn password_capable_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(DriverCapabilities::full());
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn backstage_flow_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::busy::busy_frame(app, ui);
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    )
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
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

/// The New Database page (phase 29, H44): selecting the blank template and
/// clicking Create queues create + open — the new database ends up open.
#[test]
fn backstage_new_page_creates_from_a_template() {
    let mut app = mock_app();
    app.dispatch(Command::CloseDatabase);
    let path =
        std::env::temp_dir().join(format!("datacraft-kittest-{}-new.mdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    app.new_database_path = path.to_string_lossy().into_owned();
    let mut harness = backstage_flow_harness(app);

    harness.get_by_label("New").click();
    harness.run();
    harness.get_by_label("Blank database — 0 tables").click();
    harness.run();
    harness.get_by_label("Create Database").click();
    // Create + open each take a busy frame, and the open returns to Home.
    harness.run_steps(10);

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
    let mut harness = backstage_flow_harness(app);

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

/// The Save copy flow queues its command through the busy seam; the
/// completion notice lands in the status line.
#[test]
fn backstage_save_copy_dispatches_one_command() {
    let mut app = password_capable_app();
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    app.save_as_path = "copy.mdb".to_owned();
    let mut harness = backstage_flow_harness(app);
    harness.run();
    harness.get_by_label("Save Copy").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(dump.contains("Copy saved to copy.mdb"), "{dump}");
}
