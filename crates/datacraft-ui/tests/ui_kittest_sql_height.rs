//! SQL window height budget (step 91.2, item M110) — a focused harness:
//! the result grid inside the floating window is height-bounded. A small
//! result never grows the window toward full screen (the review finding:
//! the window animated taller "as if the query were still producing
//! rows"), a large result scrolls inside the bound. The window rect rides
//! the layout probe (the H52/T10 seam).

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn app_with_rows(row_count: usize) -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "T",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Value", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    let rows: Vec<Vec<FieldValue>> = (0..row_count)
        .map(|i| {
            vec![
                FieldValue::Integer(i as i64),
                FieldValue::Text(format!("v{i}")),
            ]
        })
        .collect();
    port.with_row_data("T", rows);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "t.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleSqlWindow);
    app.set_sql_text("SELECT * FROM T");
    app
}

fn sql_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            datacraft_ui::sql_window::sql_window(app, ui);
        },
        app,
    );
    harness.set_size(egui::vec2(960.0, 600.0));
    harness
}

/// The window rect the paint site records under the stable probe id.
fn sql_window_rect(rects: &[(String, egui::Rect)]) -> egui::Rect {
    rects
        .iter()
        .find(|(id, _)| id == "sql.window")
        .map(|(_, rect)| *rect)
        .expect("the SQL window records its probe rect")
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    use egui_kittest::kittest::NodeT;
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

/// Running a small (4-row) statement keeps the window at its size — the
/// result area takes exactly its rows, no growth toward full screen.
#[test]
fn a_four_row_result_keeps_the_sql_window_at_its_size() {
    let mut harness = sql_harness(app_with_rows(4));
    harness.run();
    let before = sql_window_rect(&layout_probe::drain());
    harness.get_by_label("Run").click();
    for _ in 0..5 {
        harness.run();
    }
    let after = sql_window_rect(&layout_probe::drain());
    assert!(
        after.height() < 560.0,
        "the window must not grow toward full screen: {before:?} -> {after:?}"
    );
    assert!(
        after.height() <= before.height() + 170.0,
        "a 4-row result adds only its rows: {before:?} -> {after:?}"
    );
    harness.get_by_label_contains("4 result rows");
}

/// A large result scrolls inside the bound: the window stays at its size,
/// the row count names the full result, and rows beyond the bound stay
/// virtualized (not painted, not in the tree).
#[test]
fn a_large_result_scrolls_inside_the_bounded_area() {
    let mut harness = sql_harness(app_with_rows(60));
    harness.run();
    harness.get_by_label("Run").click();
    for _ in 0..5 {
        harness.run();
    }
    let after = sql_window_rect(&layout_probe::drain());
    assert!(
        after.height() < 560.0,
        "60 rows must scroll inside the bound, not grow the window: {after:?}"
    );
    let dump = tree_dump(&harness);
    assert!(dump.contains("60 result rows"), "the result is present:\n{dump}");
    assert!(
        !dump.contains("v59"),
        "rows beyond the bound stay virtualized:\n{dump}"
    );
}
