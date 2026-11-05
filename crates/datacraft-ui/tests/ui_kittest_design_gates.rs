//! Capability gates on the design-time schema edits (step 42.2, item
//! L18): on a source whose driver cannot alter the schema, the design
//! view's edit controls render disabled with the localized reason
//! instead of dispatching into a `Failed` toast, and the backstage
//! Create Database button gates on the same availability map. Data
//! assertions on the semantic tree — never pixels.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::snapshot::TabSnapshot;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with an open design tab over the given capabilities.
fn design_app(caps: DriverCapabilities) -> (App, TabSnapshot) {
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
    let tab = app.snapshot().tabs[0].clone();
    (app, tab)
}

fn design_harness(app: App, tab: TabSnapshot) -> Harness<'static, (App, TabSnapshot)> {
    Harness::new_ui_state(
        move |ui, (app, tab)| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::design::design_ui(app, tab, ui);
        },
        (app, tab),
    )
}

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

/// True when a node labelled `prefix` renders disabled.
fn disabled_line(dump: &str, prefix: &str) -> bool {
    dump.lines()
        .any(|line| line.starts_with(prefix) && line.ends_with(" disabled"))
}

/// True when a node labelled `prefix` renders interactive.
fn enabled_line(dump: &str, prefix: &str) -> bool {
    dump.lines()
        .any(|line| line.starts_with(prefix) && !line.ends_with(" disabled"))
}

#[test]
fn schema_edit_controls_gate_on_the_alter_tables_capability() {
    let mut caps = DriverCapabilities::full();
    caps.alter_table = false;
    let (app, tab) = design_app(caps);
    let mut harness = design_harness(app, tab);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("the open source cannot alter the schema"),
        "the restricted source shows the reason:\n{dump}"
    );
    assert!(disabled_line(&dump, "Button|Add Field|"), "{dump}");
    assert!(disabled_line(&dump, "Button|Rename|"), "{dump}");
    assert!(disabled_line(&dump, "Button|Delete|"), "{dump}");
    assert!(disabled_line(&dump, "CheckBox|Required|"), "{dump}");

    // Contrast: an alter-capable source keeps every control interactive.
    let (app, tab) = design_app(DriverCapabilities::full());
    let mut harness = design_harness(app, tab);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("the open source cannot alter the schema"),
        "{dump}"
    );
    assert!(enabled_line(&dump, "Button|Add Field|"), "{dump}");
    assert!(enabled_line(&dump, "Button|Rename|"), "{dump}");
    assert!(enabled_line(&dump, "Button|Delete|"), "{dump}");
    assert!(enabled_line(&dump, "CheckBox|Required|"), "{dump}");
}

#[test]
fn create_database_button_gates_on_the_create_capability() {
    let mut caps = DriverCapabilities::full();
    caps.create_database = false;
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.backstage_page = datacraft_ui::backstage::BackstagePage::New;
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    );
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("the open source cannot create new databases"),
        "the restricted composition shows the reason:\n{dump}"
    );
    assert!(disabled_line(&dump, "Button|Create Database|"), "{dump}");

    // Contrast: a creation-capable composition keeps the button live
    // (still gated on path and template selection).
    let mut app = App::new(Box::new(MockPort::with_tables(vec![customers()])));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.backstage_page = datacraft_ui::backstage::BackstagePage::New;
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::backstage::backstage_ui(app, ui);
        },
        app,
    );
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("the open source cannot create new databases"),
        "{dump}"
    );
    assert!(
        disabled_line(&dump, "Button|Create Database|"),
        "no path and template selected yet:\n{dump}"
    );
}
