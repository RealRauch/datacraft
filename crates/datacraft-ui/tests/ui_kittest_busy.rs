//! Busy overlay suite (step 68.5, item M75).
//!
//! The deferred-dispatch seam paints the localized overlay in one frame,
//! runs the command in the next and keeps the UI thread untouched by
//! threads or async. The tests pin the ordering invariant (no dispatch
//! before the overlay frame was painted), the rendered overlay and the
//! exactly-once execution.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::busy::OPEN_LABEL;
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

/// Harness over the real frame seam (construction runs to stability).
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

/// Harness that only paints the overlay (no ready pickup), so the pending
/// state stays observable.
fn overlay_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::busy::busy_overlay(app, ui);
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

/// The ordering invariant: a queued command is not ready before its
/// overlay frame was painted, is picked exactly once afterwards.
#[test]
fn the_command_waits_for_the_overlay_frame() {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch_blocking(
        Command::OpenDatabase {
            path: "shop.mdb".into(),
            password: None,
            source: None,
        },
        OPEN_LABEL,
    );
    assert!(app.busy_label().is_some());
    assert!(
        app.take_ready_command().is_none(),
        "no dispatch before the overlay frame"
    );
    app.mark_busy_painted();
    assert!(
        app.take_ready_command().is_some(),
        "ready after the overlay frame"
    );
    assert!(
        app.take_ready_command().is_none(),
        "the command is picked exactly once"
    );
    assert!(app.busy_label().is_none());
}

/// The overlay paints the pending action's localized label.
#[test]
fn the_overlay_renders_the_pending_action() {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch_blocking(
        Command::OpenDatabase {
            path: "shop.mdb".into(),
            password: None,
            source: None,
        },
        OPEN_LABEL,
    );
    let harness = overlay_harness(app);
    let dump = tree_dump(&harness);
    assert!(dump.contains("Working…"), "the overlay renders: {dump}");
    assert!(
        dump.contains("Opening the database…"),
        "the overlay names the action: {dump}"
    );
}

/// The real frame seam runs the deferred command exactly once.
#[test]
fn the_deferred_command_runs_exactly_once() {
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

    let mut harness = busy_harness(app);
    assert!(
        harness.state().busy_label().is_none(),
        "the construction runs the seam to completion"
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
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch_blocking(
        Command::SaveDatabaseAs {
            path: "copy.mdb".into(),
            password: None,
        },
        "Writing the database copy…",
    );
    let harness = busy_harness(app);

    assert!(
        harness.state().busy_label().is_none(),
        "the failure clears the overlay"
    );
    assert!(
        harness.state().last_error().is_some(),
        "the failure lands in the notice history"
    );
}
