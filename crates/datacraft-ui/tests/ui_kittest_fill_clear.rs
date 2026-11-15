//! Fill and clear suite (step 86.3, item M101).
//!
//! Ctrl+D fills the selection from its top row — through the H61 batch
//! on an atomic-batch driver, as one direct `UpdateCells` write
//! otherwise, and **one** undo step in both shapes. Delete over an
//! extended selection clears the cells to NULL (staged on a batch
//! driver); Delete over a collapsed cell stays the record deletion of
//! step 40.3.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::clipboard::ScriptedClipboard;
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

/// App with the Customers tab open over `rows` records, `batch` forcing
/// the atomic-batch capability (the mock default writes directly).
fn app(rows: usize, batch: bool) -> (App, TabId) {
    let data: Vec<Vec<FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                FieldValue::Integer(index as i64),
                FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    if batch {
        let mut caps = datacraft_core::port::DriverCapabilities::full();
        caps.atomic_batch = true;
        port.with_capabilities(caps);
    }
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(ScriptedClipboard::default()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
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

fn harness(app: App) -> Harness<'static, App> {
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

/// Ctrl+D fills the selected rectangle from its top row; on a batch
/// driver the fill stages as the H61 batch.
#[test]
fn ctrl_d_fills_the_selection_from_its_top_row_staged() {
    let (app, tab) = app(5, true);
    let mut harness = harness(app);
    harness.run();

    // Select the name column over rows 0..3, top row carries "alpha0".
    harness.get_by_label("alpha0").click();
    harness.run();
    harness
        .get_by_label("alpha2")
        .click_modifiers(egui::Modifiers::SHIFT);
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::D);
    harness.run();

    let staged = harness
        .state()
        .staged_rows(tab)
        .expect("the fill stages into the edit buffer");
    assert_eq!(
        staged.row_image(1),
        Some(&[FieldValue::Integer(1), FieldValue::Text("alpha0".into())][..])
    );
    assert_eq!(
        staged.row_image(2),
        Some(&[FieldValue::Integer(2), FieldValue::Text("alpha0".into())][..])
    );
    assert_eq!(
        staged.row_image(3),
        Some(&[FieldValue::Integer(3), FieldValue::Text("alpha3".into())][..]),
        "rows outside the selection keep their image"
    );
}

/// Without the batch capability the fill is one direct `UpdateCells`
/// write, and Ctrl+Z restores every row — the fill is one undo step.
#[test]
fn a_direct_fill_is_one_undo_step() {
    let (app, tab) = app(5, false);
    let mut harness = harness(app);
    harness.run();

    harness.get_by_label("alpha0").click();
    harness.run();
    harness
        .get_by_label("alpha2")
        .click_modifiers(egui::Modifiers::SHIFT);
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::D);
    harness.run();

    let rows = harness.state().rows_for(tab).expect("loaded rows");
    assert_eq!(rows[1][1], FieldValue::Text("alpha0".into()));
    assert_eq!(rows[2][1], FieldValue::Text("alpha0".into()));
    assert!(
        !harness.state().has_staged_edits(tab),
        "no batch driver — the fill writes directly"
    );

    // The fill's inverse rides the bus: one Undo restores every row
    // (the Ctrl+Z key itself is pinned by the shortcuts suite).
    harness.state_mut().dispatch(Command::Undo);
    harness.run();
    let rows = harness.state().rows_for(tab).expect("loaded rows");
    assert_eq!(rows[1][1], FieldValue::Text("alpha1".into()));
    assert_eq!(rows[2][1], FieldValue::Text("alpha2".into()));
}

/// Delete over an extended selection stages NULLs into the batch
/// (delete-to-NULL); the id column here is not required, so the mock
/// keeps the tri-state NULL (M24).
#[test]
fn delete_over_an_extended_selection_clears_to_null_staged() {
    let (app, tab) = app(5, true);
    let mut harness = harness(app);
    harness.run();

    // The selector click arms the grid and keeps the cursor away; the
    // cell clicks then build the rectangle.
    harness.get_by_label("     4").click();
    harness.run();
    harness.get_by_label("alpha0").click();
    harness.run();
    harness
        .get_by_label("alpha1")
        .click_modifiers(egui::Modifiers::SHIFT);
    harness.run();

    harness.key_press(egui::Key::Delete);
    harness.run();

    let staged = harness
        .state()
        .staged_rows(tab)
        .expect("the clear stages into the edit buffer");
    assert_eq!(
        staged.row_image(0),
        Some(&[FieldValue::Integer(0), FieldValue::Null][..])
    );
    assert_eq!(
        staged.row_image(1),
        Some(&[FieldValue::Integer(1), FieldValue::Null][..])
    );
}
