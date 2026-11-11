//! Ribbon toggle-state suite (step 69.2, item M79).
//!
//! The three window toggles expose their state as the button's selected
//! flag (AccessKit `selected`), so assistive tech and the visual accent
//! read the same snapshot truth.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn ribbon_harness(mut app: App) -> Harness<'static, App> {
    app.active_ribbon_tab = "database_tools";
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::ribbon::ribbon_ui(app, ui);
        },
        app,
    )
}

/// The AccessKit toggle state of one button (egui maps `selected` onto
/// `toggled`).
fn toggle_state(harness: &Harness<'static, App>, label: &str) -> String {
    format!(
        "{:?}",
        harness.get_by_label(label).accesskit_node().toggled()
    )
}

/// A closed window renders as not toggled.
#[test]
fn the_sql_toggle_renders_closed() {
    let app = App::new(Box::new(MockPort::new()));
    let mut harness = ribbon_harness(app);
    harness.run();
    assert_eq!(
        toggle_state(&harness, "SQL"),
        "Some(False)",
        "a closed SQL window leaves the toggle unpressed"
    );
}

/// An open window renders as toggled — the snapshot flag is the source.
#[test]
fn the_sql_toggle_renders_open() {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::ToggleSqlWindow);
    let mut harness = ribbon_harness(app);
    harness.run();
    assert_eq!(
        toggle_state(&harness, "SQL"),
        "Some(True)",
        "an open SQL window presses the toggle"
    );
}
