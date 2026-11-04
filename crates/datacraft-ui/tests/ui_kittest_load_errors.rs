//! Load-failure surfacing contract (step 39.4, item M31) — a failing port
//! read must surface a precise, localized reason in the affected view
//! instead of the endless "Loading rows…" state.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger))],
    )
}

fn failing_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_read_failure("disk on fire");
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app
}

fn tree_dump(harness: &Harness<'static, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        lines.push(format!("{:?}|{label}|{value}", node.role()));
    }
    lines.join("\n")
}

#[test]
fn a_failing_row_read_shows_the_reason_instead_of_loading() {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            if let Some(tab) = app.snapshot().tabs.first().cloned() {
                datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
            }
        },
        failing_app(),
    );
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("could not be loaded"),
        "the failure reason is surfaced: {dump}"
    );
    assert!(
        !dump.contains("Loading rows"),
        "the endless loading state is gone: {dump}"
    );
}
