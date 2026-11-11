//! Busy overlay suite (step 68.5, item M75).
//!
//! The deferred-dispatch seam paints the localized overlay in one frame,
//! runs the command in the next and keeps the UI thread untouched by
//! threads or async. The tests step frame by frame and assert on app state
//! plus the AccessKit tree.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn notes() -> TableDef {
    TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    )
}

/// App with the Notes tab open.
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

fn busy_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::busy::busy_frame(app, ui);
            ui.label("content");
        },
        app,
    )
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut text = String::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        if label.is_empty() {
            text.push_str(&node.value().unwrap_or_default());
        } else {
            text.push_str(&label);
        }
        text.push('|');
    }
    text
}

/// The overlay is painted one frame before the command runs; the command
/// executes exactly once.
#[test]
fn the_overlay_precedes_the_deferred_command_and_it_runs_once() {
    let (mut app, tab) = notes_app();
    app.dispatch_blocking(
        Command::InsertRow {
            table: "Notes".into(),
            values: vec![FieldValue::Text("first".into())],
        },
        "Saving the record…",
    );
    assert_eq!(
        app.rows_for(tab).map(Vec::len),
        Some(0),
        "the request only queues the command"
    );
    assert!(app.busy_label().is_some());

    let mut harness = busy_harness(app);
    harness.step();
    assert!(
        harness.state().busy_label().is_some(),
        "frame 1 keeps the command pending"
    );
    assert_eq!(
        harness.state().rows_for(tab).map(Vec::len),
        Some(0),
        "frame 1 only paints the overlay"
    );
    assert!(
        tree_dump(&harness).contains("Working…"),
        "the overlay is on screen: {}",
        tree_dump(&harness)
    );

    harness.step();
    assert!(
        harness.state().busy_label().is_none(),
        "frame 2 runs the command and clears the overlay"
    );
    assert_eq!(
        harness.state().rows_for(tab).map(Vec::len),
        Some(1),
        "the deferred command ran"
    );

    harness.step();
    assert_eq!(
        harness.state().rows_for(tab).map(Vec::len),
        Some(1),
        "it never runs twice"
    );
}

/// A failing deferred command also clears the overlay and reports through
/// the notice history.
#[test]
fn a_failing_deferred_command_clears_the_overlay() {
    let app = App::new(Box::new(MockPort::new()));
    let mut app = app;
    app.dispatch_blocking(
        Command::SaveDatabaseAs {
            path: "copy.mdb".into(),
            password: None,
        },
        "Writing the database copy…",
    );

    let mut harness = busy_harness(app);
    harness.step();
    assert!(harness.state().busy_label().is_some());
    harness.step();

    assert!(
        harness.state().busy_label().is_none(),
        "the failure clears the overlay"
    );
    assert!(
        harness.state().last_error().is_some(),
        "the failure lands in the notice history"
    );
}
