//! Lifecycle-navigation suite (step 69.3, item M80).
//!
//! The shell follows the action: an open leaves the backstage and shows
//! Home; a create activates the new document in its design mode. Without
//! a database the shell lands on the backstage (step 75.1, item L41).

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::state::TabMode;
use datacraft_ui::app::App;
use datacraft_ui::backstage::BackstagePage;
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

fn open_app() -> App {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![customers()])));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn ribbon_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::ribbon::ribbon_ui(app, ui);
        },
        app,
    )
}

/// An open returns from the backstage to the Home tab.
#[test]
fn opening_a_database_returns_to_home() {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![customers()])));
    app.active_ribbon_tab = "file";
    app.backstage_page = BackstagePage::Open;
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    assert_eq!(
        app.active_ribbon_tab, "home",
        "opening a database returns to Home"
    );
    assert_eq!(app.backstage_page, BackstagePage::Home);
}

fn active_document(app: &App) -> (String, String, TabMode) {
    let snapshot = app.snapshot();
    let active = snapshot.active_tab.expect("a document is active");
    let tab = snapshot.tabs.iter().find(|tab| tab.id == active).unwrap();
    (tab.object_kind.clone(), tab.object_name.clone(), tab.mode)
}

/// The ribbon's Form Design action activates the new form's design tab.
#[test]
fn form_design_activates_the_new_document() {
    let app = open_app();
    let mut harness = ribbon_harness(app);
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    harness.get_by_label("Form Design").click();
    harness.run();

    let (kind, name, mode) = active_document(harness.state());
    assert_eq!(kind, "form");
    assert_eq!(name, "Form1");
    assert_eq!(mode, TabMode::Design);
}

/// The report action activates the new report's design tab.
#[test]
fn report_design_activates_the_new_document() {
    let app = open_app();
    let mut harness = ribbon_harness(app);
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    harness.get_by_label("Report Design").click();
    harness.run();

    let (kind, name, mode) = active_document(harness.state());
    assert_eq!(kind, "report");
    assert_eq!(name, "Report1");
    assert_eq!(mode, TabMode::Design);
}

/// Renders the real four-region shell (the same `eframe::App::ui` entry the
/// binary runs) so the landing surface is observed, not only the state.
fn full_shell_harness(app: App) -> Harness<'static, App> {
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

/// Labels of every labelled node in the rendered tree, `|`-separated.
fn tree_labels(harness: &Harness<'static, App>) -> String {
    harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

/// A cold start without a database shows the File backstage instead of the
/// empty workspace (step 75.1, item L41); the Back row returns to the
/// ribbon, so Options/Info and the workspace stay reachable.
#[test]
fn cold_start_without_a_database_shows_the_file_backstage() {
    let app = App::new(Box::new(MockPort::new()));
    assert_eq!(
        app.active_ribbon_tab, "file",
        "a cold start lands on the File tab"
    );
    let mut harness = full_shell_harness(app);
    harness.run();
    let labels = tree_labels(&harness);
    assert!(
        labels.contains("Samples"),
        "the backstage is rendered: {labels}"
    );
    assert!(
        labels.contains("Back"),
        "the backstage nav is rendered: {labels}"
    );
    assert!(
        !labels.contains("Navigation Pane"),
        "the empty workspace is not painted: {labels}"
    );

    harness.get_by_label("Back").click();
    harness.run();
    assert_eq!(
        harness.state().active_ribbon_tab,
        "home",
        "the Back row returns to the ribbon"
    );
    let labels = tree_labels(&harness);
    assert!(
        labels.contains("Navigation Pane"),
        "the workspace is reachable again: {labels}"
    );
}

/// Closing the last database returns to the backstage (step 75.1, item
/// L41) — the open→Home transition (M80) keeps its existing pin.
#[test]
fn closing_the_last_database_returns_to_the_backstage() {
    let mut app = open_app();
    assert_eq!(app.active_ribbon_tab, "home");
    app.dispatch(Command::CloseDatabase);
    assert_eq!(
        app.active_ribbon_tab, "file",
        "closing the last database returns to the backstage"
    );
    assert_eq!(app.backstage_page, BackstagePage::Home);

    let mut harness = full_shell_harness(app);
    harness.run();
    let labels = tree_labels(&harness);
    assert!(
        labels.contains("Samples"),
        "the backstage is rendered: {labels}"
    );
    assert!(
        !labels.contains("Navigation Pane"),
        "the empty workspace is not painted: {labels}"
    );
}
