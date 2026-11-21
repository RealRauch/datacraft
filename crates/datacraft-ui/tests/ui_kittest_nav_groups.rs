//! Nav-pane groups and details-view suite (step 92.5, item L48).
//!
//! The selection's "New group from N selected" action opens the name
//! dialog; Create lists the group under Custom Groups with its objects.
//! The group persists per database in `ui.json`. The header dropdown
//! switches to the flat Object Details view and back.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use datacraft_ui::settings::Settings;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn group_app() -> App {
    let port = MockPort::with_tables(vec![
        TableDef::new(
            "Customers",
            vec![FieldDef::new("Name", FieldKind::Text { max_len: 40 })],
        ),
        TableDef::new(
            "ServiceOrders",
            vec![FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger))],
        ),
    ]);
    let mut app = App::new(Box::new(port));
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
            // The language static is process-wide; pin English so the
            // label queries stay deterministic.
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

/// The rect of the leftmost node carrying `label` — the nav row (the tab
/// strip may carry the same label once a table is open).
fn row_rect(harness: &Harness<'static, App>, label: &str) -> egui::Rect {
    harness
        .get_all_by_label(label)
        .map(|node| node.rect())
        .min_by(|left, right| left.min.x.total_cmp(&right.min.x))
        .expect("a node carries the label")
}

fn click_row(harness: &mut Harness<'static, App>, label: &str, modifiers: egui::Modifiers) {
    let pos = row_rect(harness, label).center();
    harness.hover_at(pos);
    harness.event(egui::Event::ModifiersChanged(modifiers));
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    harness.run();
    harness.run();
}

fn right_click_row(harness: &mut Harness<'static, App>, label: &str) {
    let pos = row_rect(harness, label).center();
    harness.hover_at(pos);
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    harness.run();
}

/// "New group from 2 selected" opens the name dialog; Create lists the
/// group under Custom Groups with both objects (step 92.5, item L48).
#[test]
fn a_group_creates_from_the_selection_and_lists_its_objects() {
    let mut harness = shell_harness(group_app());
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    click_row(&mut harness, "Customers", egui::Modifiers::COMMAND);
    click_row(&mut harness, "ServiceOrders", egui::Modifiers::COMMAND);
    right_click_row(&mut harness, "Customers");
    harness.get_by_label("New group from 2 selected").click();
    harness.run();
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Group name")
        .set_text_value_accesskit("Mine");
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Mine").is_some(),
        "the group header lists under Custom Groups"
    );
    assert!(
        harness.get_all_by_label("Customers").count() >= 2,
        "the group carries its object rows beside the section rows"
    );
    assert!(
        harness.query_by_label("Custom Groups").is_some(),
        "the groups section header is visible"
    );
}

/// The created group persists per database path in `ui.json` — the
/// sidecar and the database file never see it (step 92.5, item L48).
#[test]
fn the_group_persists_per_database_in_ui_json() {
    let dir = std::env::temp_dir().join(format!("datacraft-navgroups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    let mut app = group_app();
    app.set_settings_path(Some(path.clone()));
    let mut harness = shell_harness(app);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    click_row(&mut harness, "Customers", egui::Modifiers::COMMAND);
    click_row(&mut harness, "ServiceOrders", egui::Modifiers::COMMAND);
    right_click_row(&mut harness, "Customers");
    harness.get_by_label("New group from 2 selected").click();
    harness.run();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Group name")
        .set_text_value_accesskit("Mine");
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    harness.run();

    harness.state_mut().persist_settings().expect("save");
    let loaded = Settings::load(&path);
    let prefs = &loaded.nav["shop.mdb"];
    assert_eq!(prefs.groups.len(), 1, "the group persisted");
    assert_eq!(prefs.groups[0].name, "Mine");
    assert_eq!(prefs.groups[0].objects.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The header dropdown switches to the flat Object Details view and back;
/// the switch persists per database (step 92.5, item L48).
#[test]
fn the_details_view_lists_every_object_flat() {
    let dir = std::env::temp_dir().join(format!("datacraft-navview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    let mut app = group_app();
    app.set_settings_path(Some(path.clone()));
    let mut harness = shell_harness(app);
    harness.set_size(egui::vec2(1280.0, 800.0));
    harness.run();

    harness.get_by_label("All Access Objects").click();
    harness.run();
    harness.get_by_label("Object Details").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Tables").is_none(),
        "the tree sections are gone in the details view"
    );
    assert!(
        harness.query_by_label("Customers").is_some()
            && harness.query_by_label("ServiceOrders").is_some(),
        "every object lists flat"
    );

    harness.state_mut().persist_settings().expect("save");
    let loaded = Settings::load(&path);
    assert_eq!(loaded.nav["shop.mdb"].view, datacraft_ui::nav_groups::NavView::Details);

    harness.get_by_label("Object Details").click();
    harness.run();
    harness.get_by_label("All Access Objects").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Tables").is_some(),
        "the tree view returns"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
