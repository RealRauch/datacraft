//! Clipboard suite (step 86.2, item M101).
//!
//! Copy builds the TSV over the selection through the scripted clipboard
//! port; paste writes a fitting matrix through the tab's normal edit
//! path — staged into the H61 batch on an atomic-batch driver, direct
//! row writes otherwise — and rejections (shape, value) leave database
//! and buffer untouched: validation precedes every cell change. The TSV
//! conversion itself is pinned by the core contract suite
//! (`clipboard.rs`).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::{DriverCapabilities, MockPort};
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

/// App with the Customers tab open over `rows` records (names carry
/// their index), the scripted clipboard injected. `batch` forces the
/// atomic-batch capability (the mock's honest default is `false`, so
/// single writes run by default — step 85's coordination is pinned in
/// both shapes).
fn multi_row_table_app(rows: usize, batch: bool, clipboard: ScriptedClipboard) -> (App, TabId) {
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
        let mut caps = DriverCapabilities::full();
        caps.atomic_batch = true;
        port.with_capabilities(caps);
    }
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(clipboard));
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

/// Ctrl+C writes the selection's TSV to the clipboard port.
#[test]
fn ctrl_c_copies_the_selection_as_tsv() {
    let (app, _tab) = multi_row_table_app(6, false, ScriptedClipboard::default());
    let mut harness = datasheet_harness(app);
    harness.run();

    // A rectangle over the name column, rows 0..5.
    harness.get_by_label("alpha0").click();
    harness.run();
    harness
        .get_by_label("alpha4")
        .click_modifiers(egui::Modifiers::SHIFT);
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::C);
    harness.run();
    assert_eq!(
        harness.state().clipboard_text().as_deref(),
        Some("alpha0\nalpha1\nalpha2\nalpha3\nalpha4"),
        "the selection copies as one TSV column"
    );
}

/// Ctrl+V stages a fitting matrix into the edit buffer from the anchor
/// on an atomic-batch driver; the values parse against the column kinds.
#[test]
fn ctrl_v_stages_a_fitting_matrix_from_the_anchor() {
    let (app, tab) = multi_row_table_app(
        6,
        true,
        ScriptedClipboard::with_text("10\tgold\n20\tsilver"),
    );
    let mut harness = datasheet_harness(app);
    harness.run();

    // The selector click moves the cursor away from row 1 so the cell
    // click cannot arm the in-place editor; the cell click anchors the
    // paste at (1, 0) — the id column takes the integers, name the text.
    harness.get_by_label("     4").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::V);
    harness.run();

    let staged = harness
        .state()
        .staged_rows(tab)
        .expect("the paste stages into the edit buffer");
    assert_eq!(
        staged.row_image(1),
        Some(&[FieldValue::Integer(10), FieldValue::Text("gold".into())][..])
    );
    assert_eq!(
        staged.row_image(2),
        Some(&[FieldValue::Integer(20), FieldValue::Text("silver".into())][..])
    );
    assert_eq!(
        staged.row_image(3),
        Some(&[FieldValue::Integer(3), FieldValue::Text("alpha3".into())][..]),
        "rows outside the matrix keep their image"
    );
}

/// Without the atomic-batch capability (the mock's honest default) the
/// paste writes the matrix directly through one `UpdateRow` per target
/// row — the pre-85 path stays intact.
#[test]
fn ctrl_v_writes_directly_without_the_batch_capability() {
    let (app, tab) = multi_row_table_app(
        6,
        false,
        ScriptedClipboard::with_text("10\tgold\n20\tsilver"),
    );
    let mut harness = datasheet_harness(app);
    harness.run();

    harness.get_by_label("     4").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::V);
    harness.run();

    assert!(
        !harness.state().has_staged_edits(tab),
        "no batch driver — the paste does not stage"
    );
    let rows = harness.state().rows_for(tab).expect("the loaded rows");
    assert_eq!(
        rows[1],
        vec![FieldValue::Integer(10), FieldValue::Text("gold".into())]
    );
    assert_eq!(
        rows[2],
        vec![FieldValue::Integer(20), FieldValue::Text("silver".into())]
    );
}

/// A matrix that does not fit the grid from the anchor on is rejected
/// before any cell changes — the buffer stays empty.
#[test]
fn a_paste_that_does_not_fit_is_rejected_before_any_change() {
    // A 2x3 matrix cannot fit the two-column grid from any anchor.
    let (app, tab) = multi_row_table_app(6, true, ScriptedClipboard::with_text("a\tb\tc\nd\te\tf"));
    let mut harness = datasheet_harness(app);
    harness.run();

    harness.get_by_label("     4").click();
    harness.run();
    harness.get_by_label("1").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::V);
    harness.run();

    let error = harness.state().last_error().expect("a precise rejection");
    assert!(
        error.contains("only") && error.contains("fit"),
        "the rejection names the fit: {error}"
    );
    assert!(
        !harness.state().has_staged_edits(tab),
        "nothing staged — the rejection precedes every cell change"
    );
}

/// A matrix with a value the column kind rejects is rejected as a whole
/// before any cell changes (validation first).
#[test]
fn a_paste_with_a_bad_value_is_rejected_before_any_change() {
    // "abc" does not parse against the LongInteger id column.
    let (app, tab) = multi_row_table_app(6, true, ScriptedClipboard::with_text("abc"));
    let mut harness = datasheet_harness(app);
    harness.run();

    harness.get_by_label("     4").click();
    harness.run();
    harness.get_by_label("0").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::V);
    harness.run();

    let error = harness.state().last_error().expect("a precise rejection");
    assert!(
        error.contains("id"),
        "the rejection names the column: {error}"
    );
    assert!(
        !harness.state().has_staged_edits(tab),
        "nothing staged — one bad cell rejects the whole paste"
    );
}
