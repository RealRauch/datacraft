//! Migration-readiness window (phase 103, step 103.2; item M134).
//!
//! Opening the window runs `AnalyzeApp` over the real bus once and the
//! window renders every report section read-only: effort summary,
//! unsupported features, the VBA estimate, the linked-table inventory
//! and the per-object list. The kittest drives a `MockPort` serving the
//! fixture-shaped content (the dependency direction forbids the jet
//! crate in the UI).

use datacraft_core::command::Command;
use datacraft_core::port::{
    LinkKind, LinkedTable, MockPort, NativeObject, NativeObjectKind,
};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn port() -> MockPort {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_vba(VbaProject {
        modules: vec![VbaModule {
            name: "Clean".to_owned(),
            kind: VbaModuleKind::Standard,
            source: "Attribute VB_Name = \"Clean\"\nSub A()\n  x = 1\nEnd Sub\n".to_owned(),
        }],
    });
    port.with_native_objects(vec![NativeObject {
        name: "LegacyForm".to_owned(),
        kind: NativeObjectKind::Form,
    }]);
    port.with_macros(vec![datacraft_core::macro_def::MacroDef {
        name: "AutoExec".to_owned(),
        actions: Vec::new(),
        definition_decoded: false,
    }]);
    port.with_linked_tables(vec![LinkedTable {
        name: "Remote".to_owned(),
        kind: LinkKind::Odbc,
        source: None,
        connect: Some("ODBC;DSN=bench;UID=***".to_owned()),
        foreign_name: Some("dbo.Remote".to_owned()),
        flags: 0,
    }]);
    port
}

fn open_app() -> App {
    let mut app = App::new(Box::new(port()));
    app.dispatch(Command::OpenDatabase {
        path: "readiness.accdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleReadinessWindow);
    app
}

fn harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::readiness_window::readiness_window(app, ui);
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

/// Opening the window dispatches `AnalyzeApp` and renders every report
/// section: effort summary, unsupported features, the VBA estimate, the
/// link inventory and the per-object list with rings and statuses.
#[test]
fn the_open_window_runs_the_analysis_and_renders_every_section() {
    let mut harness = harness(open_app());
    harness.run();

    let dump = tree_dump(&harness);
    for section in [
        "Migration readiness",
        "Effort summary",
        "Unsupported features",
        "VBA estimate",
        "Linked tables",
        "Objects",
    ] {
        assert!(dump.contains(section), "section {section:?} missing:\n{dump}");
    }
    assert!(
        dump.contains("1 of 4 objects run"),
        "the effort summary counts the objects:\n{dump}"
    );
    assert!(
        dump.contains("3 not covered"),
        "the effort summary counts the gaps:\n{dump}"
    );
    assert!(
        dump.contains("never executed"),
        "the unsupported list names the macro boundary:\n{dump}"
    );
    assert!(
        dump.contains("never resolved"),
        "the unsupported list names the link boundary:\n{dump}"
    );
    assert!(
        dump.contains("datacraft-import"),
        "the unsupported list names the migration path:\n{dump}"
    );
    assert!(
        dump.contains("100%"),
        "the VBA estimate reports the candidate share:\n{dump}"
    );
    assert!(
        dump.contains("Clean: 4 of 4"),
        "the per-module estimate lists the candidates:\n{dump}"
    );
    assert!(dump.contains("Remote"), "the link inventory lists the link:\n{dump}");
    assert!(
        dump.contains("LegacyForm") && dump.contains("not covered"),
        "the object list carries ring and status per object:\n{dump}"
    );
    assert!(
        dump.contains("Customers") && dump.contains("runs"),
        "the object list marks the running table:\n{dump}"
    );
    assert!(harness.state().last_error().is_none());
}

/// The closed window renders nothing.
#[test]
fn the_closed_window_renders_nothing() {
    let mut app = App::new(Box::new(port()));
    app.dispatch(Command::OpenDatabase {
        path: "readiness.accdb".into(),
        password: None,
        source: None,
    });
    let mut harness = harness(app);
    harness.run();
    assert!(
        tree_dump(&harness).is_empty(),
        "the closed window paints nothing"
    );
}

/// Refresh re-runs the analysis over the bus.
#[test]
fn refresh_reruns_the_analysis() {
    let mut harness = harness(open_app());
    harness.run();
    let first = tree_dump(&harness);
    assert!(first.contains("Migration readiness"), "{first}");

    harness.get_by_label("Refresh").click();
    harness.run();
    assert!(
        tree_dump(&harness).contains("Migration readiness"),
        "the window survives a refresh"
    );
    assert!(harness.state().last_error().is_none());
}
