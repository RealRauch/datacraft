//! Datasheet selection suite (step 86.1, item M101).
//!
//! Ctrl+A, the rectangle extension from the anchor (Shift+click and
//! Shift+arrows), whole-column selection through the header menu and the
//! documented survival rule (a filter change clamps the selection into
//! the new row set). Assertions stay on the presentation state — the
//! model math itself is pinned by the core contract suite
//! (`selection.rs`).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::selection::{CellPos, Selection};
use datacraft_ui::app::App;
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

/// App with the Customers tab open over `rows` records. Names carry their
/// index so the headless suite can address cells by label.
fn multi_row_table_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("alpha{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
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

/// Ctrl+A over an armed grid selects everything loaded.
#[test]
fn ctrl_a_selects_every_loaded_cell() {
    let (app, tab) = multi_row_table_app(6);
    let mut harness = datasheet_harness(app);
    harness.run();

    // Arming the grid: a row-selector click.
    harness.get_by_label("     3").click();
    harness.run();
    assert_eq!(
        harness.state().selection(tab, 6, 2),
        Selection::collapse(0, 0),
        "no selection happened yet"
    );

    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::A);
    harness.run();
    assert_eq!(harness.state().selection(tab, 6, 2), Selection::All);
}

/// A plain click sets the anchor cell; Shift+click extends the rectangle
/// to the clicked cell, from either corner.
#[test]
fn shift_click_extends_the_rectangle_from_the_anchor() {
    let (app, tab) = multi_row_table_app(6);
    let mut harness = datasheet_harness(app);
    harness.run();

    // The selector click moves the cursor away from row 0 so the first
    // cell click cannot arm the in-place editor.
    harness.get_by_label("     3").click();
    harness.run();
    harness.get_by_label("alpha0").click();
    harness.run();
    assert_eq!(
        harness.state().selection(tab, 6, 2),
        Selection::collapse(0, 1),
        "a plain click collapses onto the cell"
    );

    harness.get_by_label("alpha4").click_modifiers(egui::Modifiers::SHIFT);
    harness.run();
    assert_eq!(
        harness.state().selection(tab, 6, 2),
        Selection::Rect {
            anchor: CellPos { row: 0, col: 1 },
            focus: CellPos { row: 4, col: 1 },
        }
    );
}

/// A plain navigation key collapses the selection onto the new cursor
/// row; a Shift-modified one extends from the current anchor.
#[test]
fn plain_arrows_collapse_and_shift_arrows_extend() {
    let (app, tab) = multi_row_table_app(6);
    let mut harness = datasheet_harness(app);
    harness.run();

    harness.get_by_label("     2").click();
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selection(tab, 6, 2), Selection::collapse(2, 0));

    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    harness.run();
    assert_eq!(
        harness.state().selection(tab, 6, 2),
        Selection::Rect {
            anchor: CellPos { row: 2, col: 0 },
            focus: CellPos { row: 3, col: 0 },
        }
    );
}

/// The header context menu selects a whole column.
#[test]
fn the_header_menu_selects_a_whole_column() {
    let (app, tab) = multi_row_table_app(6);
    let mut harness = datasheet_harness(app);
    harness.run();

    // The second header cell ("name"): the selector is 48 wide, the
    // default column width 140 — x 200 lands inside it.
    right_click(&mut harness, egui::pos2(200.0, 13.0));
    harness.run();
    harness.get_by_label("Select Column").click();
    harness.run();
    assert_eq!(
        harness.state().selection(tab, 6, 2),
        Selection::Columns {
            anchor: 1,
            focus: 1
        }
    );
}

/// The survival rule (module doc, core `selection`): a filter change
/// shrinks the row set and the selection clamps into it on the next
/// read — here a rectangle over rows 2..6 of six records after the rows
/// collapse to one.
#[test]
fn a_filter_change_clamps_the_selection_into_the_new_row_set() {
    let (mut app, tab) = multi_row_table_app(6);
    app.set_selection(
        tab,
        Selection::Rect {
            anchor: CellPos { row: 2, col: 0 },
            focus: CellPos { row: 5, col: 1 },
        },
    );
    app.dispatch(Command::SetTableFilter {
        table: "Customers".into(),
        field: "name".into(),
        criteria: Some("Like \"alpha0*\"".into()),
    });
    // The display now carries one row; the read-time clamp shrinks the
    // rectangle onto it.
    assert_eq!(
        app.selection(tab, 1, 2),
        Selection::Rect {
            anchor: CellPos { row: 0, col: 0 },
            focus: CellPos { row: 0, col: 1 },
        }
    );
}

/// Raw secondary click with the pointer parked at `pos` (the freeze
/// suite's pattern).
fn right_click(harness: &mut Harness<'static, App>, pos: egui::Pos2) {
    harness.hover_at(pos);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
}
