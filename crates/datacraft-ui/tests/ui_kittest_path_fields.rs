//! The path-field contract (step 91.3, item L53): a realistic long path
//! is identifiable at the default field size — the chosen file name
//! renders as its own prominent identity row beside the field (the full
//! path rides the hover, whose text equals the field value). Pinned for
//! the import window and every backstage path field.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

const LONG_PATH: &str =
    "C:\\Users\\someuser\\Downloads\\data-exchange\\database-access-ready.mdb";
const FILE_NAME: &str = "database-access-ready.mdb";

fn app() -> App {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
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

/// A node whose label is exactly the file name — the identity row, not
/// the field's raw value (the value line carries the whole path).
fn asserts_identity_row(dump: &str, surface: &str) {
    assert!(
        dump.contains(&format!("|{FILE_NAME}|")),
        "the {surface} field names the chosen file as its own row:\n{dump}"
    );
}

/// The import window's source field identifies the chosen file at the
/// default window size.
#[test]
fn the_import_source_field_names_the_chosen_file() {
    let mut app = app();
    app.set_import_source(LONG_PATH);
    app.dispatch(Command::ToggleImportWindow);
    let mut harness = shell_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    asserts_identity_row(&dump, "import source");
}

/// Every backstage path field (open, new database, save-a-copy, sidecar)
/// carries the same identity row.
#[test]
fn every_backstage_path_field_names_the_chosen_file() {
    let pages: [(BackstagePage, &str, fn(&mut App), fn(&mut App)); 4] = [
        (
            BackstagePage::Open,
            "open",
            |app: &mut App| app.open_path = LONG_PATH.to_owned(),
            |_| {},
        ),
        (
            BackstagePage::New,
            "new database",
            |app: &mut App| app.new_database.path = LONG_PATH.to_owned(),
            |app: &mut App| app.new_database.kind = Some(datacraft_ui::backstage::NewDatabaseKind::Blank),
        ),
        (
            BackstagePage::Save,
            "save-a-copy",
            |app: &mut App| app.save_as_path = LONG_PATH.to_owned(),
            |_| {},
        ),
        (
            BackstagePage::Save,
            "sidecar",
            |app: &mut App| app.sidecar_path = LONG_PATH.to_owned(),
            |_| {},
        ),
    ];
    for (page, surface, seed, prepare) in pages {
        let mut app = app();
        seed(&mut app);
        prepare(&mut app);
        app.backstage_page = page;
        let mut harness = shell_harness(app);
        harness.run();
        let dump = tree_dump(&harness);
        asserts_identity_row(&dump, surface);
    }
}
