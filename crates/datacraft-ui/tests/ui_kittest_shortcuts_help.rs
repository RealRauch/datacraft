//! Shortcuts help suite (step 68.4, item M77).
//!
//! F1 toggles the localized shortcuts page through the real key handler;
//! the page renders the exhaustive table and closes on its Close button.

use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn help_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::keys::handle(app, ui.ctx());
            datacraft_ui::shortcuts_help::shortcuts_help(app, ui);
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

/// F1 opens the page with the full table; Close closes it.
#[test]
fn f1_opens_the_shortcuts_page_and_close_closes_it() {
    let app = App::new(Box::new(MockPort::new()));
    let mut harness = help_harness(app);
    harness.run();

    harness.key_press(egui::Key::F1);
    harness.run();
    assert!(harness.state().shortcuts_help_open(), "F1 opens the page");

    let dump = tree_dump(&harness);
    assert!(dump.contains("Keyboard Shortcuts"), "{dump}");
    for keys in ["Ctrl+S", "Ctrl+W", "Ctrl+Z", "Ctrl+Y", "Esc", "F1"] {
        assert!(dump.contains(keys), "the table lists {keys}: {dump}");
    }

    harness.get_by_label("Close").click();
    harness.run();
    assert!(
        !harness.state().shortcuts_help_open(),
        "the Close button closes the page"
    );
}
