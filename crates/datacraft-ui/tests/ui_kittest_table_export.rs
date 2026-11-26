//! Table export affordance (phase 99, step 99.2; item M130).
//!
//! The datasheet carries one export entry per format: a click parks the
//! save dialog, the answer dispatches `ExportTable`, and the completion
//! reaches the notice history. Assertions stay on the written file, the
//! notice text and the bus state — never on pixels.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with the Customers datasheet open over two records.
fn export_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Bob".into())],
        ],
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

fn export_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            app.poll_dialogs();
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

fn target(stem: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-ui-table-export-{}-{stem}",
        std::process::id()
    ))
}

fn notices(harness: &Harness<'static, App>) -> Vec<String> {
    harness
        .state()
        .notice_entries()
        .map(|entry| entry.text())
        .collect()
}

#[test]
fn the_datasheet_exports_the_table_as_csv_through_the_dialog() {
    let path = target("out.csv");
    let _ = std::fs::remove_file(&path);
    let mut app = export_app();
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().save(Some(path.to_str().expect("utf-8 path"))),
    ));
    let mut harness = export_harness(app);
    harness.set_size(egui::vec2(1100.0, 800.0));
    harness.run();
    harness.get_by_label("Export CSV…").click();
    // The parked dialog answers on a later frame, then the export runs.
    harness.run();
    harness.run();
    let entries = notices(&harness);
    assert!(
        entries
            .iter()
            .any(|entry| entry.contains("Table exported to") && entry.contains("2 rows")),
        "{entries:?}"
    );
    let text = String::from_utf8(std::fs::read(&path).expect("the CSV exists")).expect("UTF-8");
    assert_eq!(text, "id,name\n1,Ada\n2,Bob\n");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_datasheet_exports_the_table_as_json_through_the_dialog() {
    let path = target("out.json");
    let _ = std::fs::remove_file(&path);
    let mut app = export_app();
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().save(Some(path.to_str().expect("utf-8 path"))),
    ));
    let mut harness = export_harness(app);
    harness.set_size(egui::vec2(1100.0, 800.0));
    harness.run();
    harness.get_by_label("Export JSON…").click();
    harness.run();
    harness.run();
    let entries = notices(&harness);
    assert!(
        entries
            .iter()
            .any(|entry| entry.contains("Table exported to") && entry.contains("2 rows")),
        "{entries:?}"
    );
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the JSON exists"))
            .expect("valid JSON");
    assert_eq!(doc["table"], "Customers");
    assert_eq!(doc["rows"][0]["name"], "Ada");
    let _ = std::fs::remove_file(&path);
}
