//! Shared helpers of the layout suites (the file-size split of step 92.2):
//! the table fixture, the full-shell frame renderer and the probe lookup.
//! Two suites include this via `#[path]` — every helper is `pub` so both
//! see it.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

/// An app with the two-catalog fixture tables open.
pub fn app_with_tables() -> App {
    let port = MockPort::with_tables(vec![
        TableDef::new(
            "ServiceOrders",
            vec![
                FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("Description", FieldKind::Text { max_len: 50 }),
            ],
        ),
        TableDef::new(
            "Customers",
            vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
        ),
    ]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
        source: None,
    });
    app
}

/// Renders the full shell at `size` and drains the rect log plus the
/// semantic tree.
pub fn shell_frame(app: App, size: egui::Vec2) -> (Vec<(String, egui::Rect)>, String) {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
            layout_probe::record("probe.window", ui.max_rect());
        },
        app,
    );
    harness.set_size(size);
    harness.run();
    harness.run();
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}{disabled}", node.role()));
    }
    (layout_probe::drain(), lines.join("\n"))
}

/// Looks one recorded paint rect up by its probe id.
pub fn panel_rect(rects: &[(String, egui::Rect)], id: &str) -> egui::Rect {
    rects
        .iter()
        .find(|(name, _)| name == id)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("rect {id:?} recorded"))
}
