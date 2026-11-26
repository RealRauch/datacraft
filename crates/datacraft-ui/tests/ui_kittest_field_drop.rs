//! Drag-and-drop field binding in the designer (phase 102, step 102.4;
//! item M133): dragging a field row from the palette onto the canvas
//! places the bound control at the drop point; a plain click keeps adding
//! at the next free slot. The core contract is pinned by
//! `datacraft-core/tests/field_drop.rs`.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers_app() -> App {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )])));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    app.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Form, "Card"),
        mode: TabMode::Design,
    });
    app
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            layout_probe::clear();
            datacraft_ui::form_design::form_design_ui(app, &tab, ui);
        },
        app,
    );
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();
    harness
}

fn control_rects(app: &App) -> Vec<datacraft_core::canvas::Rect> {
    app.snapshot().database.clone().expect("open").forms[0]
        .controls
        .iter()
        .map(|control| control.rect)
        .collect()
}

/// One pointer drag with intermediate moves, so egui sees a real drag.
fn drag(harness: &mut Harness<'_, App>, from: egui::Pos2, to: egui::Pos2) {
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=4 {
        let t = step as f32 / 4.0;
        harness.hover_at(from + (to - from) * t);
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    harness.run();
}

/// Dropping the `name` field on the canvas places a bound text box at the
/// drop point.
#[test]
fn a_dragged_field_lands_at_the_drop_point() {
    let mut harness = harness(customers_app());
    let canvas = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "form.design.canvas")
        .map(|(_, rect)| *rect)
        .expect("the canvas records its rect");
    let field_row = harness.get_by_label("name  (Short Text)").rect().center();
    drag(
        &mut harness,
        field_row,
        canvas.min + egui::vec2(200.0, 100.0),
    );

    let db = harness.state().snapshot().database.clone().expect("open");
    let control = db.forms[0].controls.first().expect("a control landed");
    assert_eq!(control.name, "name");
    assert_eq!(
        control.rect,
        datacraft_core::canvas::Rect {
            x: 200.0,
            y: 104.0,
            width: 120.0,
            height: 24.0,
        },
        "the drop point snaps to the grid"
    );
    assert_eq!(
        control_rects(harness.state()).len(),
        1,
        "one drop, one control"
    );
}

/// A click on the same row still adds at the next free slot (the click
/// path stays unchanged).
#[test]
fn a_click_still_adds_at_the_next_free_slot() {
    let mut harness = harness(customers_app());
    let field_row = harness.get_by_label("active  (Yes/No)").rect().center();
    harness.hover_at(field_row);
    harness.drag_at(field_row);
    harness.drop_at(field_row);
    harness.run();
    harness.run();

    let db = harness.state().snapshot().database.clone().expect("open");
    assert_eq!(db.forms[0].controls.len(), 1);
    assert_eq!(db.forms[0].controls[0].name, "active");
}
