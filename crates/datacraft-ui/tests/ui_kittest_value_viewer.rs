//! Cell value viewer suite (step 86.4, item M102).
//!
//! Shift+F2, Alt+click and the cell context menu open the zoom window:
//! JSON indents only when it parses (invalid JSON stays raw), binary
//! shows a bounded hex view naming the truncation, and an editable memo
//! becomes a multiline editor writing through the H61 batch. Opening
//! never modifies the database.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::clipboard::ScriptedClipboard;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn catalog() -> TableDef {
    TableDef::new(
        "Catalog",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("doc", FieldKind::Memo),
            FieldDef::new("blob", FieldKind::Binary(BinaryKind::OleObject)),
        ],
    )
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

/// `write = false` simulates a read-only source, so the zoom window
/// renders its read-only forms instead of the multiline editor.
fn app_with(row: Vec<FieldValue>, write: bool) -> (App, TabId) {
    let mut port = MockPort::with_tables(vec![catalog()]);
    port.with_row_data("Catalog", vec![row]);
    let mut caps = datacraft_core::port::DriverCapabilities::full();
    if !write {
        caps.write_rows = false;
    } else {
        caps.atomic_batch = true;
    }
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port)).with_clipboard(Box::new(ScriptedClipboard::default()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Catalog",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

/// Shift+F2 opens the viewer on the cursor's cell: a JSON memo indents
/// (the pretty form carries a newline+indent), and Esc closes it.
#[test]
fn shift_f2_opens_the_json_viewer_and_esc_closes() {
    let (app, tab) = app_with(
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("{\"a\": 1, \"b\": [2, 3]}".into()),
            FieldValue::Binary(vec![0xDE, 0xAD]),
        ],
        false,
    );
    let mut harness = harness(app);
    harness.run();

    // The selector click arms the grid and puts the cursor on row 0; the
    // doc column becomes the focus cell via a plain cell click.
    harness.get_by_label("     1").click();
    harness.run();
    harness.get_by_label("{\"a\": 1, \"b\": [2, 3]}").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F2);
    harness.run();

    // Pretty-printed JSON: the indented continuation only exists in the
    // pretty form (the raw cell text has no newline).
    assert!(
        harness.query_by_label_contains("\n  \"a\": 1,").is_some(),
        "the JSON value renders indented"
    );
    assert!(
        !harness.state().has_staged_edits(tab),
        "opening the viewer modifies nothing"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        harness.query_by_label_contains("\n  \"b\": [").is_none(),
        "escape closed the viewer"
    );
}

/// Invalid JSON stays raw: no pretty form appears.
#[test]
fn invalid_json_stays_raw() {
    let (app, _tab) = app_with(
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("{not json".into()),
            FieldValue::Binary(vec![]),
        ],
        false,
    );
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     1").click();
    harness.run();
    harness.get_by_label("{not json").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F2);
    harness.run();

    // The window's read-only body renders the raw text; the title proves
    // the window (the grid cell carries the same raw label).
    assert!(
        harness.query_by_label_contains("Value — doc").is_some(),
        "the zoom window opens"
    );
    assert!(
        harness.query_by_label_contains("\n  \"not").is_none(),
        "invalid JSON never pretty-prints"
    );
}

/// A binary value shows the bounded hex view; over the bound the window
/// names the truncation. Alt+click on the cell opens it.
#[test]
fn binary_values_show_bounded_hex() {
    let big: Vec<u8> = [0xDE, 0xAD].into_iter().cycle().take(600).collect();
    let (app, _tab) = app_with(
        vec![
            FieldValue::Integer(1),
            FieldValue::Text(String::new()),
            FieldValue::Binary(big),
        ],
        false,
    );
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     1").click();
    harness.run();
    harness
        .get_by_label("<binary: 600 bytes>")
        .click_modifiers(egui::Modifiers::ALT);
    harness.run();

    assert!(
        harness.query_by_label_contains("DE AD DE AD").is_some(),
        "the hex view renders byte pairs"
    );
    assert!(
        harness
            .query_by_label_contains("the first 512 of 600 bytes")
            .is_some(),
        "the window names the truncation"
    );
}

/// An editable memo becomes a multiline editor; Apply writes through the
/// H61 batch (the driver here stages).
#[test]
fn an_editable_memo_edits_multiline_through_the_batch() {
    let (app, tab) = app_with(
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("line one\nline two".into()),
            FieldValue::Binary(vec![]),
        ],
        true,
    );
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("     1").click();
    harness.run();
    harness.get_by_label("line one\nline two").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F2);
    harness.run();

    // The viewer is a multiline editor for the editable memo: a text
    // input exists beyond the grid's (the editor carries the draft).
    // The viewer's multiline editor: the "Cell value" label resolves to
    // the text input carrying the draft.
    assert!(
        harness
            .query_by_role_and_label(
                eframe::egui::accesskit::Role::MultilineTextInput,
                "Cell value",
            )
            .is_some(),
        "the viewer carries the multiline editor"
    );

    let new_text = "rewritten\nwith two lines";
    harness
        .state_mut()
        .set_value_editor_draft(tab, new_text.to_owned());
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();

    let staged = harness
        .state()
        .staged_rows(tab)
        .expect("the edit stages into the batch");
    assert_eq!(
        staged.row_image(0),
        Some(
            &[
                FieldValue::Integer(1),
                FieldValue::Text(new_text.to_owned()),
                FieldValue::Binary(vec![]),
            ][..]
        )
    );
}
