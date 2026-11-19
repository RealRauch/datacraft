//! The AutoNumber new-record contract (step 91.6, item M114): the gate
//! renders before input — the AutoNumber cell is disabled while the text
//! cells stay editable, and the row hint names the contract precisely.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn app_with_table() -> App {
    let table = TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    );
    let mut port = MockPort::with_tables(vec![table]);
    port.with_row_data(
        "Customers",
        vec![vec![FieldValue::Integer(1), FieldValue::Text("Lena".into())]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app
}

fn datasheet_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
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
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// The AutoNumber cell renders its gate before input: disabled, while
/// the text cell of the same row stays editable.
#[test]
fn the_autonumber_new_record_cell_is_gated() {
    let mut harness = datasheet_harness(app_with_table());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.lines()
            .any(|line| line.starts_with("TextInput|Id|") && line.ends_with(" disabled")),
        "the AutoNumber cell is disabled:\n{dump}"
    );
    assert!(
        dump.lines()
            .any(|line| line.starts_with("TextInput|Name|") && !line.ends_with(" disabled")),
        "the text cell stays editable:\n{dump}"
    );
}

/// The row hint names the AutoNumber contract precisely — the reviewer's
/// finding: "does not say whether a manual value may be entered".
#[test]
fn the_new_record_hint_names_the_autonumber_contract() {
    let mut harness = datasheet_harness(app_with_table());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("AutoNumber cells reject input"),
        "the hint names the rejection:\n{dump}"
    );
    assert!(
        dump.contains("engine assigns the value"),
        "the hint names the assignment:\n{dump}"
    );
}
