//! Excel workbooks as read-only sources through the composed stack
//! (phase 96, step 96.3; item M127).
//!
//! The driver docks through `DetectingPort` like the CSV proof: the
//! snapshot names the driver and family, lists the sheets as tables,
//! reports every write affordance as unavailable with its reason, and the
//! datasheet renders the typed rows — while every write command is
//! refused before the port.

use std::path::{Path, PathBuf};

use datacraft_core::availability::DisabledReason;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{DatabaseFormat, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_storage::detecting::DetectingPort;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("excelFixture.xlsx")
}

fn workbook_driver() -> Box<dyn StoragePort> {
    Box::new(datacraft_xlsx::ExcelPort::new())
}

fn core() -> Core {
    let mut core = Core::new(Box::new(
        DetectingPort::new().with_fallback(workbook_driver),
    ));
    let events = core.dispatch(Command::OpenDatabase {
        path: fixture(),
        password: None,
        source: None,
    });
    assert!(
        matches!(events.first(), Some(Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    core
}

#[test]
fn the_composite_names_the_driver_family_and_sheets() {
    let core = core();
    let database = core.inspect().database.expect("open");
    assert_eq!(database.driver.as_str(), "xlsx");
    assert_eq!(database.format, DatabaseFormat::Excel);
    assert_eq!(
        database
            .tables
            .iter()
            .map(|table| table.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Items", "Summary"]
    );
    assert!(database.capabilities.read_tables);
    assert!(!database.capabilities.write_rows);
    assert!(!database.capabilities.create_table);
    assert!(!database.capabilities.drop_table);
    assert_eq!(
        database.availability.get("datasheet.new_record"),
        Some(&DisabledReason::WriteRows)
    );
    assert_eq!(
        database.availability.get("maintenance.compact"),
        Some(&DisabledReason::Compact)
    );
    let rows = core
        .read_rows(
            "Items",
            datacraft_core::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read rows");
    assert_eq!(rows[0][0], FieldValue::Text("Ada".into()));
    assert_eq!(rows[1][5], FieldValue::Text("hello".into()));
}

#[test]
fn every_write_command_is_refused_before_the_port() {
    let mut core = core();
    let cases = [
        (
            Command::InsertRow {
                table: "Items".into(),
                values: vec![FieldValue::Text("x".into())],
            },
            DisabledReason::WriteRows,
        ),
        (
            Command::DropTable {
                table: "Items".into(),
            },
            DisabledReason::DropTables,
        ),
        (
            Command::CreateTable {
                definition: datacraft_core::schema::TableDef::access_default("New"),
            },
            DisabledReason::CreateTables,
        ),
        (Command::CompactDatabase, DisabledReason::Compact),
    ];
    for (command, expected) in cases {
        let events = core.dispatch(command);
        match events.as_slice() {
            [Event::Failed { code, .. }] => {
                assert_eq!(
                    code,
                    &FailureCode::Capability(expected),
                    "the refusal names its reason"
                );
            }
            other => panic!("expected a capability refusal, got {other:?}"),
        }
    }
}

fn harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::workspace::workspace_ui(app, ui);
        },
        app,
    )
}

#[test]
fn the_datasheet_renders_the_workbook_rows() {
    let mut app = App::new(Box::new(
        DetectingPort::new().with_fallback(workbook_driver),
    ));
    app.dispatch(Command::OpenDatabase {
        path: fixture(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Items",
    )));
    let mut harness = harness(app);
    harness.run();
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
    let text = labels.join("|");
    assert!(text.contains("Ada"), "the first row renders: {text}");
    assert!(text.contains("Grace"), "the second row renders: {text}");
    assert!(
        text.contains("hello"),
        "the text cell renders its value: {text}"
    );
}
