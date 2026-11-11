//! Ribbon tooltip suite (step 68.2, item M74).
//!
//! The registry is only half the contract: the paint code must actually
//! render the localized sentence on hover. The assertion walks the
//! AccessKit tree after a real hover.

use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn ribbon_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            // The production delay (0.5 s) would need simulated wall time;
            // the paint path is identical with a zero delay.
            ui.style_mut().interaction.tooltip_delay = 0.0;
            datacraft_ui::ribbon::ribbon_ui(app, ui);
        },
        app,
    )
}

/// Hovering the Navigation Pane button renders the registry sentence.
#[test]
fn hovering_a_ribbon_action_renders_its_tooltip() {
    let app = App::new(Box::new(MockPort::new()));
    let mut harness = ribbon_harness(app);
    harness.run();

    let button = harness.get_by_label("Navigation Pane");
    harness.hover_at(button.rect().center());
    harness.run();

    let dump: String = harness
        .root()
        .children_recursive()
        .map(|node| {
            let node = node.accesskit_node();
            format!(
                "{:?}|{}|{}",
                node.role(),
                node.label().unwrap_or_default(),
                node.value().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        dump.contains("Show or hide the navigation pane."),
        "the hovered action must render its registry tooltip: {dump}"
    );
}
