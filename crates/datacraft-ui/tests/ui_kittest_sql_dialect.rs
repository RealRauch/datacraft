//! SQL window dialect surfaces (phase 98, step 98.2; item M129): the
//! window names the engine behind RunSql, offers the query-plan toggle on
//! native sources only, and the run path stays the same command.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort, Support};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn app(native: bool) -> App {
    let mut port = MockPort::new();
    port.with_capabilities(DriverCapabilities {
        native_sql: if native { Support::Yes } else { Support::No },
        ..DriverCapabilities::full()
    });
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.sqlite".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleSqlWindow);
    app
}

fn harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::sql_window::sql_window(app, ui);
        },
        app,
    )
}

fn dump(harness: &Harness<'static, App>) -> String {
    let mut labels = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        if let Some(label) = node.label() {
            labels.push(label.to_owned());
        }
        if let Some(value) = node.value() {
            labels.push(value.to_owned());
        }
    }
    labels.join("|")
}

#[test]
fn a_native_source_names_its_dialect_and_offers_the_plan() {
    let mut harness = harness(app(true));
    harness.run();
    let text = dump(&harness);
    assert!(text.contains("native SQLite · read-only"), "{text}");
    assert!(
        text.contains("EXPLAIN QUERY PLAN run on the file itself"),
        "{text}"
    );
    assert!(text.contains("Query plan"), "{text}");
    let plan = harness.get_by_label("Query plan");
    assert!(!plan.accesskit_node().is_disabled(), "native enables it");
}

#[test]
fn a_shadow_source_keeps_the_shadow_wording_and_disables_the_plan() {
    let mut harness = harness(app(false));
    harness.run();
    let text = dump(&harness);
    assert!(text.contains("one SELECT/WITH statement"), "{text}");
    assert!(text.contains("SQLite semantics"), "{text}");
    assert!(
        harness
            .get_by_label("Query plan")
            .accesskit_node()
            .is_disabled(),
        "the shadow engine has no plan surface"
    );
}
