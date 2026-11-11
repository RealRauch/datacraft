//! Undo/Redo keyboard suite (step 68.1, item M73).
//!
//! Ctrl+Z and Ctrl+Y travel through the real `keys::handle` seam into the
//! bus: the shortcut dispatches exactly one Undo/Redo when the history
//! allows it and nothing when it does not (the QAT rule — an empty history
//! must not synthesize a failure notice).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;

fn notes() -> TableDef {
    TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    )
}

/// App with the Notes tab open over an empty table.
fn notes_app() -> (App, TabId) {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![notes()])));
    app.dispatch(Command::OpenDatabase {
        path: "notes.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Notes",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

/// Harness over the real shortcut handler plus the datasheet (the keys
/// need a focused egui context; the datasheet keeps the row cache visible).
fn shortcut_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::keys::handle(app, ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

fn cached_rows(harness: &Harness<'static, App>, tab: TabId) -> Option<usize> {
    harness.state().rows_for(tab).map(Vec::len)
}

/// One insert through the bus, then Ctrl+Z undoes it and Ctrl+Y redoes it.
#[test]
fn control_z_undoes_and_control_y_redoes_the_last_row_write() {
    let (mut app, tab) = notes_app();
    app.dispatch(Command::InsertRow {
        table: "Notes".into(),
        values: vec![FieldValue::Text("first".into())],
    });
    let mut harness = shortcut_harness(app);
    harness.run();
    assert_eq!(cached_rows(&harness, tab), Some(1), "the insert is visible");
    assert_eq!(harness.state().snapshot().undo_depth, 1);

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Z);
    harness.run();

    assert_eq!(
        cached_rows(&harness, tab),
        Some(0),
        "Ctrl+Z dispatches one Undo over the bus"
    );
    assert_eq!(harness.state().snapshot().redo_depth, 1);

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Y);
    harness.run();

    assert_eq!(
        cached_rows(&harness, tab),
        Some(1),
        "Ctrl+Y dispatches one Redo over the bus"
    );
}

/// An empty history answers no failure: the key does nothing and the
/// status bar stays clean (the QAT rule, extended to the keyboard).
#[test]
fn control_z_with_an_empty_history_does_nothing() {
    let (app, tab) = notes_app();
    let mut harness = shortcut_harness(app);
    harness.run();
    assert_eq!(harness.state().snapshot().undo_depth, 0);

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Z);
    harness.run();

    assert_eq!(cached_rows(&harness, tab), Some(0));
    assert!(
        harness.state().last_error().is_none(),
        "an empty history must not synthesize a failure: {:?}",
        harness.state().last_error()
    );
}
