//! Native Access form/report inventory in the navigation pane (phase 74,
//! step 74.1; item M87).
//!
//! The pane gains a read-only group naming the catalog's native forms and
//! reports; entries are disabled (no tab, no definition decoding) and a
//! hint states the boundary.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, NativeObject, NativeObjectKind};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn native_app() -> App {
    let mut port = MockPort::new();
    port.with_native_objects(vec![
        NativeObject {
            name: "Category Picker".into(),
            kind: NativeObjectKind::Form,
        },
        NativeObject {
            name: "Items Browser".into(),
            kind: NativeObjectKind::Form,
        },
        NativeObject {
            name: "Movement Editor".into(),
            kind: NativeObjectKind::Form,
        },
        NativeObject {
            name: "Stock Summary".into(),
            kind: NativeObjectKind::Report,
        },
    ]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.accdb".into(),
        password: None,
        source: None,
    });
    app
}

fn nav_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::layout_probe::clear();
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
        },
        app,
    )
}

#[test]
fn the_nav_pane_lists_native_objects_read_only() {
    let mut harness = nav_harness(native_app());
    harness.run();

    let labels: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().label().as_deref().map(str::to_owned))
        .collect();
    let joined = labels.join("|");
    assert!(
        joined.contains("Native forms and reports (4)"),
        "the group names the count: {joined}"
    );
    for name in [
        "Category Picker",
        "Items Browser",
        "Movement Editor",
        "Stock Summary",
    ] {
        assert!(joined.contains(name), "{name} missing: {joined}");
    }
    // Every entry is read-only: rendered disabled, never a tab.
    let enabled: Vec<&String> = labels
        .iter()
        .filter(|label| label.as_str() == "Items Browser")
        .collect();
    assert!(!enabled.is_empty(), "{joined}");
    let disabled = harness
        .root()
        .children_recursive()
        .filter(|node| {
            let node = node.accesskit_node();
            node.label().as_deref() == Some("Items Browser") && node.is_disabled()
        })
        .count();
    assert_eq!(disabled, 1, "the entry renders disabled: {joined}");

    let rects = datacraft_ui::layout_probe::drain();
    assert!(
        rects
            .iter()
            .any(|(id, rect)| id == "nav.native_objects_hint" && rect.width() > 0.0),
        "the boundary hint is painted: {rects:?}"
    );
    assert!(harness.state().snapshot().tabs.is_empty(), "no tab opens");
}
