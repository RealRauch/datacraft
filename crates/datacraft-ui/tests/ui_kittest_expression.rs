//! Expression window (step 41.2, item M46).
//!
//! The window dispatches `RunExpression` without row context over the
//! real bus and renders the result value or the precise failure. Both
//! renderings are pinned through the semantic tree; the empty-scope
//! evaluation needs no open database, so the suite runs bare.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn expression_app() -> App {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::ToggleExpressionWindow);
    app
}

fn expression_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::expression_window::expression_window(app, ui);
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

/// A successful run reaches the bus and the window renders the result
/// value.
#[test]
fn the_expression_window_renders_the_evaluated_result() {
    let mut app = expression_app();
    app.set_expression_text("1 + 1");
    let mut harness = expression_harness(app);
    harness.run();
    harness.get_by_label("Run").click();
    harness.run();

    let dump = tree_dump(&harness);
    assert!(
        dump.contains("= 2"),
        "the window renders the result:\n{dump}"
    );
    assert!(
        harness.state().last_error().is_none(),
        "{:?}",
        harness.state().last_error()
    );
}

/// A failing run renders the precise parse/evaluation reason instead of
/// a stale or empty result.
#[test]
fn the_expression_window_renders_the_precise_failure() {
    let mut app = expression_app();
    app.set_expression_text("1 +");
    let mut harness = expression_harness(app);
    harness.run();
    harness.get_by_label("Run").click();
    harness.run();

    let error = harness
        .state()
        .expression_error()
        .expect("the precise failure is cached")
        .to_owned();
    assert!(!error.is_empty());
    let dump = tree_dump(&harness);
    assert!(
        dump.contains(&error),
        "the window renders the failure:\n{dump}"
    );
    assert!(harness.state().last_error().is_some());
    assert!(
        !dump.contains("= 2"),
        "a failed run replaces the previous result:\n{dump}"
    );
}

/// A customers table with one record: the row-context fixture (step
/// 60.1, item L27) — the core resolves `[name]` against the record.
fn customers_app() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![FieldValue::Text("alpha".to_owned())]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleExpressionWindow);
    app
}

/// Picking a table and record gives the expression the row's scope: the
/// field name resolves to the record's value (step 60.1, item L27).
#[test]
fn a_row_context_resolves_the_record_fields() {
    let mut harness = expression_harness(customers_app());
    harness.run();

    harness.get_by_label("Row context").click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Customers"),
        "the picker lists the open tables:\n{dump}"
    );
    harness.get_by_label("Customers").click();
    harness.run();
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Row")
        .set_text_value_accesskit("0");
    harness.run();

    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Expression")
        .set_text_value_accesskit("[name]");
    harness.run();
    harness.get_by_label("Run").click();
    harness.run();

    let output = harness.state().expression_output().map(str::to_owned);
    assert!(
        output
            .as_deref()
            .is_some_and(|output| output.contains("alpha")),
        "the row context resolved the record's field: {output:?}"
    );
}

/// Clearing the picker returns to the empty scope: the same expression
/// then answers the unknown-field failure — the contextless evaluation
/// stays the default state (step 60.1, item L27).
#[test]
fn clearing_the_context_returns_to_the_empty_scope() {
    let mut harness = expression_harness(customers_app());
    harness.run();
    harness.get_by_label("Row context").click();
    harness.run();
    harness.get_by_label("Customers").click();
    harness.run();
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Row")
        .set_text_value_accesskit("0");
    harness.run();

    harness.get_by_label("Row context").click();
    harness.run();
    harness.get_by_label("no row context").click();
    harness.run();

    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Expression")
        .set_text_value_accesskit("[name]");
    harness.run();
    harness.get_by_label("Run").click();
    harness.run();

    let error = harness.state().expression_error().map(str::to_owned);
    assert!(
        error.is_some(),
        "the empty scope rejects the unknown field again: {error:?}"
    );
}
