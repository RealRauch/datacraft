//! The Import File window flow (step 58.2, item M50): the ribbon button
//! opens the window, the scripted dialog seam feeds the source, the
//! preview renders the inferred fields, and one Import click fills the
//! table over the bus and closes the window. The capability-disabled
//! rendering is pinned in the same suite.

use std::path::{Path, PathBuf};
use std::process::id;

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn sample_csv() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-import-ui-{}", id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("inventory.csv");
    std::fs::write(&path, "sku,label,qty\n1,bolt,50\n2,nut,120\n").expect("write the csv");
    path
}

fn writable_app(source: &Path) -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )]);
    let _ = &mut port;
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().file(Some(source.to_string_lossy().as_ref())),
    ));
    app
}

fn shell_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

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

/// The full flow (step 58.2, item M50): ribbon button opens the window,
/// the scripted dialog feeds the source, the preview renders the inferred
/// fields, and Import fills the table over the bus and closes the window.
#[test]
fn the_import_file_window_imports_a_csv_over_the_bus() {
    let source = sample_csv();
    let mut harness = shell_harness(writable_app(&source));
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    // The button lives in the External Data ribbon tab.
    harness.get_by_label("External Data").click();
    harness.run();
    harness.get_by_label("Import File").click();
    harness.run();
    let open = harness.state().snapshot().import_window_open;
    assert!(
        open,
        "the ribbon button opens the window; dump:
{}",
        tree_dump(&harness)
    );

    harness.get_by_label("Browse…").click();
    harness.run();
    let preview = harness
        .state()
        .snapshot()
        .import_preview
        .clone()
        .expect("the scripted dialog fed the source and the preview parsed");
    assert_eq!(preview.table, "inventory", "the stem is the suggested name");
    assert_eq!(preview.fields.len(), 3);
    assert_eq!(preview.total_rows, 2);
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("rows detected"),
        "the preview names the row count:\n{dump}"
    );

    // The ribbon group caption is also labelled "Import" — the role query
    // pins the button.
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Import")
        .click();
    harness.run();
    let snapshot = harness.state().snapshot();
    assert!(
        !snapshot.import_window_open,
        "a full import closes the window over the bus"
    );
    let table = snapshot
        .database
        .as_ref()
        .expect("the database is open")
        .tables
        .iter()
        .find(|table| table.name == "inventory")
        .expect("the imported table is catalogued");
    assert_eq!(table.fields.len(), 3, "the header named the fields");
}

/// The capability-disabled rendering (step 58.2, item M50): a source that
/// cannot create tables renders the ribbon button muted with the reason —
/// and the window never opens.
#[test]
fn the_import_button_renders_disabled_for_a_read_only_source() {
    let source = sample_csv();
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "name",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    let mut caps = DriverCapabilities::full();
    caps.create_table = false;
    caps.write_rows = false;
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().file(Some(source.to_string_lossy().as_ref())),
    ));
    let mut harness = shell_harness(app);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    harness.get_by_label("External Data").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Button|Import File| disabled"),
        "the import button is disabled with the reason:\n{dump}"
    );
    assert!(
        !harness.state().snapshot().import_window_open,
        "a disabled button opens nothing"
    );
    assert!(
        !dump.contains("Source file"),
        "the window is not in the tree:\n{dump}"
    );
}
