//! Freeze-columns suite (step 40.2, item M39).
//!
//! The header context menu freezes and unfreezes leading columns; frozen
//! columns stay pinned in header and body while the remaining columns
//! scroll horizontally (injected `MouseWheel` events drive the real
//! scroll path). Assertions stay on the semantic tree and node rects —
//! the header itself is painted text, so the right-click target derives
//! from the first row-selector node's rect (fixed harness size, default
//! DPI).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn wide_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("a", FieldKind::Text { max_len: 50 }),
            FieldDef::new("b", FieldKind::Text { max_len: 50 }),
            FieldDef::new("c", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// App with the Customers tab open over four columns; the row texts carry
/// their index so cells stay uniquely addressable by label.
fn wide_table_app(rows: usize) -> (App, TabId) {
    let data: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index as i64),
                datacraft_core::schema::FieldValue::Text(format!("a{index}")),
                datacraft_core::schema::FieldValue::Text(format!("b{index}")),
                datacraft_core::schema::FieldValue::Text(format!("c{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![wide_table()]);
    port.with_row_data("Customers", data);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;
    (app, tab)
}

fn freeze_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            // A fixed-max-rect child ui stands in for the document panel:
            // it clips the grid so the four columns (4 x 140 + 48
            // selector) genuinely overflow horizontally and cannot grow
            // the pane.
            let pane = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            datacraft_ui::datasheet::datasheet_ui(app, &tab, &mut child);
        },
        app,
    );
    harness.set_size(egui::vec2(400.0, 300.0));
    harness
}

/// Secondary click at `pos` (the painted header has no AccessKit node).
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

/// Point inside the first column's header cell. Fixed pane coordinates:
/// AccessKit node rects of the grid body sit in the scrolled content ui's
/// raw coordinate space, so the painted header (no nodes) is addressed by
/// geometry instead. After the initial two wheel steps (~160 px) the
/// first column keeps a visible sliver around x=15; y=16 is inside the
/// 26 px header band.
fn first_header_cell(_harness: &Harness<'static, App>) -> egui::Pos2 {
    egui::pos2(15.0, 16.0)
}

/// Freezing through the header context menu arms the frozen pane; frozen
/// columns stay pinned while horizontal wheel scrolls move the rest.
#[test]
fn freezing_pins_leading_columns_while_the_rest_scrolls() {
    let (app, tab) = wide_table_app(4);
    let mut harness = freeze_harness(app);
    harness.run();

    // The wheel scrolls the area under the pointer: park it over the body
    // grid, inside the viewport. A first scroll moves every column.
    harness.hover_at(egui::pos2(200.0, 49.0));
    harness.run();
    let wheel_right = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-80.0, 0.0),
        modifiers: egui::Modifiers::default(),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    };
    harness.event(wheel_right.clone());
    harness.run();
    harness.run();
    let scrolled_before = harness.get_by_label("b0").rect();
    harness.event(wheel_right.clone());
    harness.run();
    harness.run();
    let scrolled_after = harness.get_by_label("b0").rect();
    assert!(
        scrolled_after.left() < scrolled_before.left() - 20.0,
        "the plain grid scrolls horizontally: {scrolled_before:?} -> {scrolled_after:?}"
    );

    // Freeze through the header context menu.
    let target = first_header_cell(&harness);
    right_click(&mut harness, target);
    harness.run();
    harness.get_by_label("Freeze Fields").click();
    harness.run();
    assert_eq!(harness.state().frozen_columns(tab), 1);

    // The frozen column is the first one (id), whose row-0 cell is "0".
    let frozen_before = harness.get_by_label("0").rect();
    let scrolled_before = harness.get_by_label("b0").rect();
    // Another scroll: the frozen column stays pinned, the rest moves on.
    harness.event(wheel_right.clone());
    harness.run();
    harness.run();
    let frozen_after = harness.get_by_label("0").rect();
    let scrolled_after = harness.get_by_label("b0").rect();
    assert!(
        (frozen_after.left() - frozen_before.left()).abs() < 0.5,
        "the frozen column stays pinned: {frozen_before:?} -> {frozen_after:?}"
    );
    assert!(
        scrolled_after.left() < scrolled_before.left() - 20.0,
        "the scrolled column moves left: {scrolled_before:?} -> {scrolled_after:?}"
    );

    // The context menu unfreezes again.
    let target = first_header_cell(&harness);
    right_click(&mut harness, target);
    harness.get_by_label("Unfreeze All Fields").click();
    harness.run();
    assert_eq!(harness.state().frozen_columns(tab), 0);
}
