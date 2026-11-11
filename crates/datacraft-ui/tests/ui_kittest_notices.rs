//! Messages dialog suite (step 68.3, item M76).
//!
//! The dialog lists the bounded history newest first and clears it; the
//! assertions walk the AccessKit tree and the app state, never pixels.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn notices_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::notices_dialog::notices_dialog(app, ui);
        },
        app,
    )
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    harness
        .root()
        .children_recursive()
        .map(|node| {
            let node = node.accesskit_node();
            let label = node.label().unwrap_or_default();
            if label.is_empty() {
                node.value().unwrap_or_default()
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Two failed dispatches produce two entries; Clear empties the history.
#[test]
fn the_messages_dialog_lists_the_history_and_clears_it() {
    let mut app = App::new(Box::new(MockPort::new()));
    let _ = app.dispatch(Command::Undo);
    let _ = app.dispatch(Command::Undo);
    let latest = app.last_error().map(str::to_owned).expect("a failure");
    app.open_notices();
    let mut harness = notices_harness(app);
    harness.run();

    let dump = tree_dump(&harness);
    assert!(dump.contains("Messages"), "the dialog renders: {dump}");
    assert!(dump.contains(&latest), "the history text is listed: {dump}");

    harness.get_by_label("Clear").click();
    harness.run();

    assert_eq!(
        harness.state().notice_entries().count(),
        0,
        "Clear empties the history"
    );
}
