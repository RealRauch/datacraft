//! Ribbon truth pass (step 41.5, item M49).
//!
//! "Table Design" activates the real design flow (a new Access-default
//! table opens in the design view), and "Import File" renders an honest
//! planned-feature state — no action claims a phase that has passed.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn ribbon_app() -> App {
    let app = App::new(Box::new(MockPort::new()));
    let mut app = app;
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app
}

fn ribbon_harness(mut app: App, tab: &'static str) -> Harness<'static, App> {
    app.active_ribbon_tab = tab;
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::ribbon::ribbon_ui(app, ui);
        },
        app,
    )
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// Clicking Table Design creates the Access-default table and opens it
/// in the design view — the same flow the nav pane's Design menu uses.
#[test]
fn table_design_opens_a_new_table_in_the_design_view() {
    let mut harness = ribbon_harness(ribbon_app(), "create");
    harness.run();

    harness.get_by_label("Table Design").click();
    harness.run();

    let snapshot = harness.state().snapshot();
    let database = snapshot.database.as_ref().expect("database open");
    assert!(
        database
            .tables
            .iter()
            .any(|table| table.name.starts_with("Table")),
        "the Access-default table was created"
    );
    let tab = &snapshot.tabs[0];
    assert_eq!(tab.object_kind, "table", "{tab:?}");
    assert_eq!(
        tab.mode,
        datacraft_core::state::TabMode::Design,
        "the new table opens in the design view"
    );
    assert!(harness.state().last_error().is_none());
}

/// Import File renders disabled with the planned-feature reason — no
/// phase claim remains.
#[test]
fn import_file_renders_an_honest_planned_state() {
    let mut harness = ribbon_harness(ribbon_app(), "external_data");
    harness.run();
    let import = harness.get_by_label("Import File");
    assert!(
        import.accesskit_node().is_disabled(),
        "the planned feature renders disabled"
    );

    let tooltip_text = {
        let rect = import.rect();
        harness.hover_at(rect.center());
        harness.run();
        tree_dump(&harness)
    };
    assert!(
        tooltip_text.contains("planned — not available yet"),
        "the tooltip names the planned state:\n{tooltip_text}"
    );
    assert!(
        !tooltip_text.contains("arrives in"),
        "no phase claim remains:\n{tooltip_text}"
    );
}
