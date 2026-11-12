//! Object rename surface (phase 72, step 72.4; item M81).
//!
//! The navigation pane's context menu and F2 open the rename dialog;
//! Rename dispatches exactly one `RenameObject` through the bus. Refusals
//! keep the dialog open with the precise reason; the capability-disabled
//! entry never dispatches.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn table(name: &str) -> TableDef {
    TableDef::new(
        name,
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn rename_app(caps: Option<DriverCapabilities>) -> App {
    let mut port = MockPort::with_tables(vec![table("Customers"), table("Orders")]);
    if let Some(caps) = caps {
        port.with_capabilities(caps);
    }
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    app
}

fn rename_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::keys::handle(app, ui.ctx());
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
            datacraft_ui::rename_dialog::rename_dialog(app, ui);
        },
        app,
    )
}

fn right_click(harness: &mut Harness<'static, App>, label: &str) {
    let rect = harness.get_by_label(label).rect();
    harness.hover_at(rect.center());
    let press = |pressed| egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
}

fn type_name(harness: &mut Harness<'static, App>, name: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "New name")
        .set_text_value_accesskit(name);
    harness.run();
    harness.run();
    let value = harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "New name")
        .accesskit_node()
        .value()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(value, name, "the text field received the typed name");
}

fn table_name(harness: &Harness<'static, App>) -> String {
    harness
        .state()
        .snapshot()
        .database
        .as_ref()
        .expect("open")
        .tables
        .iter()
        .find(|table| table.name != "Orders")
        .expect("the renamed table")
        .name
        .clone()
}

#[test]
fn the_nav_pane_context_menu_renames_the_open_table() {
    let mut harness = rename_harness(rename_app(None));
    harness.run();

    right_click(&mut harness, "Customers");
    harness.get_by_label("Rename").click();
    harness.run();
    harness.run();
    assert!(
        harness.state().rename_draft().is_some(),
        "the menu opens the dialog"
    );

    type_name(&mut harness, "Clients");
    harness.get_by_label("Rename").click();
    harness.run();
    harness.run();

    assert!(
        harness.state().rename_draft().is_none(),
        "success closes the dialog"
    );
    assert_eq!(table_name(&harness), "Clients");
    let tabs = harness.state().snapshot().tabs.clone();
    assert_eq!(tabs[0].object_name, "Clients");
    assert!(
        harness
            .get_by_label("Clients")
            .accesskit_node()
            .label()
            .is_some(),
        "the nav pane renders the new name"
    );
}

#[test]
fn f2_opens_the_dialog_for_the_active_tab() {
    let mut harness = rename_harness(rename_app(None));
    harness.run();

    harness.key_press(egui::Key::F2);
    harness.run();
    let draft = harness.state().rename_draft().cloned().expect("dialog");
    assert_eq!(draft.object.kind, ObjectKind::Table);
    assert_eq!(draft.object.name, "Customers");
    assert_eq!(draft.name, "Customers", "seeded with the current name");

    type_name(&mut harness, "Clients");
    harness.get_by_label("Rename").click();
    harness.run();
    harness.run();
    assert_eq!(table_name(&harness), "Clients");
}

#[test]
fn a_refused_rename_keeps_the_dialog_and_the_precise_reason() {
    let mut harness = rename_harness(rename_app(None));
    harness.run();

    harness.key_press(egui::Key::F2);
    harness.run();
    type_name(&mut harness, "orders");
    harness.get_by_label("Rename").click();
    harness.run();
    harness.run();

    let draft = harness.state().rename_draft().cloned().expect("still open");
    let error = draft.error.as_deref().unwrap_or_default();
    assert!(error.contains("duplicate"), "{error}");
    assert_eq!(table_name(&harness), "Customers");
}

#[test]
fn a_source_without_the_rename_capability_disables_the_entry() {
    let mut caps = DriverCapabilities::full();
    caps.rename_table = false;
    let mut harness = rename_harness(rename_app(Some(caps)));
    harness.run();

    right_click(&mut harness, "Customers");
    harness.run();
    let disabled: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let node = node.accesskit_node();
            let label = node.label().unwrap_or_default();
            (label == "Rename" && node.is_disabled()).then_some(label)
        })
        .collect();
    assert!(
        !disabled.is_empty(),
        "the rename entry renders disabled without the capability"
    );

    harness.get_by_label("Rename").click();
    harness.run();
    harness.run();
    assert!(
        harness.state().rename_draft().is_none(),
        "a disabled entry never opens the dialog"
    );
}
