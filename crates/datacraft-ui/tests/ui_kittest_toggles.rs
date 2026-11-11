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

/// A closed window renders as not selected.
#[test]
fn the_sql_toggle_renders_closed() {
    let app = App::new(Box::new(MockPort::new()));
    let mut harness = ribbon_harness(app);
    harness.run();
    let node = harness.get_by_label("SQL");
    assert_eq!(
        node.accesskit_node().is_selected(),
        Some(false),
        "a closed SQL window leaves the toggle unselected"
    );
}

/// An open window renders as selected — the snapshot flag is the source.
#[test]
fn the_sql_toggle_renders_open() {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::ToggleSqlWindow);
    let mut harness = ribbon_harness(app);
    harness.run();
    let node = harness.get_by_label("SQL");
    assert_eq!(
        node.accesskit_node().is_selected(),
        Some(true),
        "an open SQL window selects the toggle"
    );
}
