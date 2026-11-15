//! Shared helpers of the pending-batch suites (step 85.2/85.3, item
//! H61): the app/harness builders and the cell-addressing accessors.
//! Included by both suite binaries via `#[path]`.

#![allow(dead_code)]

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::dialogs::ScriptedDialogs;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

pub fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with the Customers tab open over four records; names carry their
/// index so cells stay uniquely addressable by label.
/// The database path for this suite lives in the session temp dir — a
/// test never writes artifacts into the source tree (LESSONS #31): a
/// sidecar save would otherwise land beside the test crate and leak into
/// every sibling suite's open path.
pub fn db_path() -> String {
    let dir = std::env::temp_dir().join(format!("datacraft-batch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join("shop.mdb").to_string_lossy().into_owned()
}

pub fn batch_app() -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..4)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    port.with_capabilities(DriverCapabilities {
        atomic_batch: true,
        ..DriverCapabilities::full()
    });
    let mut app = App::new(Box::new(port));
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    app.dispatch(Command::OpenDatabase {
        path: db_path().into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

/// Harness over the real shortcut handler plus the datasheet; the
/// focused harness stands in for the shell (dialog poll leads the frame,
/// step 81.2, item M97).
pub fn batch_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            app.poll_dialogs();
            datacraft_ui::keys::handle(app, ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::datasheet::datasheet_ui(app, &tab, ui);
        },
        app,
    )
}

/// Selects the record at zero-based `row` and arms the editor on its
/// name cell (two clicks: the selector, then the cell).
pub fn arm_editor(harness: &mut Harness<'static, App>, row: usize) {
    harness.get_by_label(&format!("{:>6}", row + 1)).click();
    harness.run();
    harness.get_by_label(&format!("alpha{row}")).click();
    harness.run();
}

/// Types `text` into the armed editor and commits with Enter.
pub fn type_and_commit(harness: &mut Harness<'static, App>, text: &str) {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
}

pub fn cached_name(harness: &Harness<'static, App>, tab: TabId, row: usize) -> String {
    harness
        .state()
        .rows_for(tab)
        .and_then(|rows| rows.get(row))
        .and_then(|values| values.get(1))
        .map(datacraft_ui::labels::value_label)
        .unwrap_or_default()
}

pub fn staged_texts(harness: &Harness<'static, App>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let n = node.accesskit_node();
            let value = n.value().unwrap_or_default();
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect()
}
