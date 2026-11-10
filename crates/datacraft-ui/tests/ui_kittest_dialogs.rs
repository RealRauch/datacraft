//! Native-dialog wiring contract (step 38.3, item M43) — a focused file so
//! the main kittest suite stays below the file-size gate. The scripted
//! `DialogHost` queues answers; every flow must dispatch exactly the same
//! single commands as before, gated by the scripted answers.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
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

fn app_with_database() -> App {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![customers()])));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn backstage_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    )
}

/// The Open page Browse buttons fill the path buffer from the scripted
/// native pickers — file and folder (the CSV directory driver).
#[test]
fn open_browse_fills_the_path_from_the_dialog() {
    let mut app = app_with_database();
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Open;
    app.set_dialogs(Box::new(
        ScriptedDialogs::new()
            .file(Some("C:/data/shop.mdb"))
            .folder(Some("C:/data/csvdir")),
    ));
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label("Browse…").click();
    harness.run();
    assert_eq!(
        harness.state().open_path,
        "C:/data/shop.mdb",
        "the file picker fills the path buffer"
    );
    harness.get_by_label("Browse Folder…").click();
    harness.run();
    assert_eq!(
        harness.state().open_path,
        "C:/data/csvdir",
        "the folder picker fills the path buffer"
    );
}

/// Deleting an object confirms natively first: a declined confirmation
/// dispatches nothing, an accepted one drops the table.
#[test]
fn delete_object_confirms_before_dropping() {
    let mut app = app_with_database();
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().confirm(false).confirm(true),
    ));
    app.delete_object(datacraft_core::ObjectKind::Table, "Customers");
    let snapshot = app.snapshot();
    let tables = snapshot.database.as_ref().expect("open").tables.clone();
    assert!(
        tables.iter().any(|table| table.name == "Customers"),
        "a declined confirmation must not dispatch the drop"
    );
    app.delete_object(datacraft_core::ObjectKind::Table, "Customers");
    let snapshot = app.snapshot();
    let tables = snapshot.database.as_ref().expect("open").tables.clone();
    assert!(
        !tables.iter().any(|table| table.name == "Customers"),
        "an accepted confirmation drops the table"
    );
}

/// Saving a copy over an existing file asks natively: declined keeps the
/// file, accepted writes the copy.
#[test]
fn save_copy_gates_the_overwrite_on_confirmation() {
    let target =
        std::env::temp_dir().join(format!("datacraft-overwrite-{}.mdb", std::process::id()));
    std::fs::write(&target, b"original").expect("seed the existing target");
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(datacraft_core::port::DriverCapabilities::full());
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.backstage_page = datacraft_ui::backstage::BackstagePage::Save;
    app.save_as_path = target.display().to_string();
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().confirm(false).confirm(true),
    ));
    let mut harness = backstage_harness(app);
    harness.run();
    harness.get_by_label("Save Copy").click();
    harness.run();
    assert_eq!(
        std::fs::read(&target).expect("target readable"),
        b"original",
        "a declined overwrite keeps the existing file"
    );
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("Copy saved to"),
        "a declined overwrite must not dispatch the copy"
    );
    harness.get_by_label("Save Copy").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Copy saved to"),
        "an accepted overwrite dispatches the copy: {dump}"
    );
    let _ = std::fs::remove_file(&target);
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
