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
        source: None,
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

/// Import File is enabled and opens the Import File window (step 58.1,
/// item M50) — the planned-state claim is gone. A source that cannot
/// import renders the availability reason instead (pinned over the full
/// shell in `ui_kittest_import.rs`).
#[test]
fn import_file_opens_the_import_window() {
    let mut harness = ribbon_harness(ribbon_app(), "external_data");
    harness.run();
    let import = harness.get_by_label("Import File");
    assert!(
        !import.accesskit_node().is_disabled(),
        "the import button is enabled for a writable source"
    );
    import.click();
    harness.run();
    assert!(
        harness.state().snapshot().import_window_open,
        "the button opens the Import File window"
    );
}
