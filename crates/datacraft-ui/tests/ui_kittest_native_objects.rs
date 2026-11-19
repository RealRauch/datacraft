//! Native Access form/report inventory in the navigation pane (phase 74,
//! step 74.1; item M87).
//!
//! The pane gains a read-only group naming the catalog's native forms and
//! reports; entries are disabled (no tab, no definition decoding) and a
//! hint states the boundary.

use datacraft_core::command::Command;
use datacraft_core::port::{MockPort, NativeObject, NativeObjectKind};
use datacraft_ui::app::App;
use eframe::egui;
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
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::layout_probe::clear();
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
        },
        app,
    );
    // Tall enough for the whole scroll content: the semantic tree culls
    // static text outside the viewport (buttons survive, labels do not —
    // the M60/M61 note), and the read-only rows are plain labels now.
    harness.set_size(egui::vec2(600.0, 1600.0));
    harness
}

#[test]
fn the_nav_pane_lists_native_objects_read_only() {
    let mut harness = nav_harness(native_app());
    harness.run();

    // Static text rides the accessibility value, not the label field —
    // read both (the M60/M61 note: buttons carry labels, labels carry
    // values).
    let labels: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let node = node.accesskit_node();
            node.label()
                .or_else(|| node.value())
                .as_deref()
                .map(str::to_owned)
        })
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
    // Every entry is read-only (step 91.7, item L56): rendered in the
    // pane's row language — a plain labelled row, never a control, never
    // a tab.
    let rows: Vec<(String, String)> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let node = node.accesskit_node();
            node.label()
                .or_else(|| node.value())
                .map(|text| (text.to_owned(), format!("{:?}", node.role())))
        })
        .collect();
    let items_browser = rows
        .iter()
        .find(|(text, _)| text == "Items Browser")
        .expect("the entry is listed");
    assert_eq!(items_browser.1, "Label", "the row is a plain label");
    // The label's own Label+TextRun pair is the pane's text language; no
    // control role (button, input, menu) may carry the name.
    assert!(
        !rows.iter().any(|(text, role)| text == "Items Browser"
            && matches!(role.as_str(), "Button" | "TextInput" | "CheckBox")),
        "the entry carries no control chrome: {rows:?}"
    );

    let rects = datacraft_ui::layout_probe::drain();
    assert!(
        rects
            .iter()
            .any(|(id, rect)| id == "nav.native_objects_hint" && rect.width() > 0.0),
        "the boundary hint is painted: {rects:?}"
    );
    assert!(harness.state().snapshot().tabs.is_empty(), "no tab opens");
}
