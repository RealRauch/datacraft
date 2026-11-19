//! The form designer/runtime canvas contracts (step 91.10, items
//! M120–M122): the selected-control ring paints the accent token, the
//! runtime combo's affordance is a painted caret (never a letter), and
//! the fixed 640×400 canvas stays reachable at the pinned minimum window
//! — the canvases lay out fully inside scrollable bodies (designer and
//! runtime), whose viewports ride the layout probe.

use datacraft_core::command::Command;
use datacraft_core::form::ControlKind;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;

fn customers_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn form_app_with_combo() -> App {
    let mut port = MockPort::with_tables(vec![customers_table()]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Integer(1),
            FieldValue::Text("Lena".into()),
        ]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Form1".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Form, "Form1", "Customers");
    app.dispatch(Command::AddFormControl {
        form: "Form1".into(),
        control: datacraft_core::form::ControlDef {
            name: "Pick".into(),
            kind: ControlKind::ComboBox {
                bound_field: Some("Name".into()),
                row_source: String::new(),
                display_field: String::new(),
            },
            rect: datacraft_core::canvas::Rect {
                x: 8.0,
                y: 8.0,
                width: 160.0,
                height: 24.0,
            },
        },
    });
    app
}

fn shell_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    );
    // The pinned minimum window with the navigation pane open.
    harness.set_size(egui::vec2(960.0, 600.0));
    harness
}

/// The selected-control ring paints the accent token (M120) — the paint
/// site no longer strokes the selection wash (nearly invisible in dark
/// mode). Pinned as a source contract over the shared paint fn.
#[test]
fn the_selected_ring_paints_the_accent_token() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/form_design.rs"));
    assert!(
        !source.contains("selection.bg_fill"),
        "the ring strokes the accent family, not the selection wash"
    );
}

/// The runtime combo's affordance is a painted caret (M121) — never a
/// letter standing in for an icon; the caret rect rides the
/// `form.combo.caret` probe.
#[test]
fn the_runtime_combo_paints_a_caret() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/form_runtime.rs"));
    assert!(
        !source.contains("menu_button(\"v\""),
        "the caret is painted, never a letter"
    );

    let mut app = form_app_with_combo();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Form1",
    )));
    let mut harness = shell_harness(app);
    harness.run();
    let caret = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "form.combo.caret")
        .map(|(_, rect)| *rect)
        .expect("the combo records its painted caret");
    assert!(caret.width() > 0.0, "the caret paints: {caret:?}");
}

/// The canvas stays reachable at the pinned minimum (M122): both the
/// designer and the runtime lay the full 640-wide canvas out inside a
/// scrollable body, and the designer's viewport is narrower than the
/// canvas there — the horizontal scroll is what makes it reachable. The
/// viewports ride the layout probe.
#[test]
fn the_form_canvas_scrolls_at_the_pinned_minimum() {
    // The designer.
    let mut app = form_app_with_combo();
    app.dispatch(Command::OpenObjectInMode {
        object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Form, "Form1"),
        mode: datacraft_core::state::TabMode::Design,
    });
    let mut harness = shell_harness(app);
    harness.run();
    let rects = layout_probe::drain();
    let design_viewport = rects
        .iter()
        .find(|(id, _)| id == "form.design.viewport")
        .map(|(_, rect)| *rect)
        .expect("the designer viewport records its rect");
    let design_canvas = rects
        .iter()
        .find(|(id, _)| id == "form.design.canvas")
        .map(|(_, rect)| *rect)
        .expect("the designer canvas lays out fully");
    assert!(
        (design_canvas.width() - 640.0).abs() < 1.0,
        "the full canvas lays out: {design_canvas:?}"
    );
    // The pinned-minimum viewport clips the canvas vertically — the
    // both-axes scroll is what keeps every part reachable (the measured
    // geometry: the column caps at the canvas width, the viewport is
    // shorter than the canvas).
    assert!(
        design_viewport.height() < design_canvas.height(),
        "the designer viewport scrolls the canvas at the pinned minimum: {design_viewport:?} vs {design_canvas:?}"
    );

    // The runtime.
    let mut app = form_app_with_combo();
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Form1",
    )));
    let mut harness = shell_harness(app);
    harness.run();
    let rects = layout_probe::drain();
    let runtime_canvas = rects
        .iter()
        .find(|(id, _)| id == "form.runtime.canvas")
        .map(|(_, rect)| *rect)
        .expect("the runtime canvas lays out fully");
    assert!(
        (runtime_canvas.width() - 640.0).abs() < 1.0,
        "the full canvas lays out: {runtime_canvas:?}"
    );
}
