//! The designers' resize handles (phase 102, step 102.2; item M133).
//!
//! A real pointer gesture on a handle must resize the selected control
//! through the shared core geometry: the dragged edge follows, the
//! opposite one stays, the result snaps and lands in the snapshot. The
//! core math itself is pinned by `datacraft-core/tests/canvas_resize.rs`;
//! this suite proves the affordance is wired in both designers.

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind, ControlProperties};
use datacraft_core::port::MockPort;
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;

fn label(name: &str, text: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: text.into() },
        rect: Rect {
            x: 40.0,
            y: 40.0,
            width: 160.0,
            height: 24.0,
        },
        properties: ControlProperties::default(),
    }
}

fn app_with(kind: ObjectKind) -> App {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    match kind {
        ObjectKind::Form => {
            app.dispatch(Command::CreateForm {
                name: "Card".into(),
            });
            app.dispatch(Command::AddFormControl {
                form: "Card".into(),
                control: label("C1", "Customers"),
            });
            app.dispatch(Command::OpenObjectInMode {
                object: ObjectId::new(ObjectKind::Form, "Card"),
                mode: TabMode::Design,
            });
            let tab = app.snapshot().tabs[0].id;
            app.set_form_selection(tab, "C1".into());
        }
        _ => {
            app.dispatch(Command::CreateReport {
                name: "Sheet".into(),
            });
            app.dispatch(Command::AddReportControl {
                report: "Sheet".into(),
                control: label("Title", "Sales"),
            });
            app.dispatch(Command::OpenObjectInMode {
                object: ObjectId::new(ObjectKind::Report, "Sheet"),
                mode: TabMode::Design,
            });
            let tab = app.snapshot().tabs[0].id;
            app.set_report_selection(tab, "Title".into());
        }
    }
    app
}

type Render = fn(&mut App, &datacraft_core::snapshot::TabSnapshot, &mut eframe::egui::Ui);

fn harness(app: App, design: Render) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            layout_probe::clear();
            design(app, &tab, ui);
        },
        app,
    );
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();
    harness
}

/// One recorded paint rect of the frame under test.
fn record(id: &str) -> egui::Rect {
    layout_probe::drain()
        .iter()
        .find(|(recorded, _)| recorded == id)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("{id} records its rect"))
}

/// Center of a painted resize handle (the probes record each square).
fn handle_pos(id: &str) -> egui::Pos2 {
    record(id).center()
}

/// One pointer drag with intermediate moves, so egui accumulates a real
/// drag delta per frame instead of a single jump.
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

fn form_control(app: &App) -> Rect {
    let db = app.snapshot().database.clone().expect("open");
    db.forms[0].controls[0].rect
}

fn report_control(app: &App) -> Rect {
    let db = app.snapshot().database.clone().expect("open");
    db.reports[0].controls[0].rect
}

/// The east handle widens the form control and keeps its left edge.
#[test]
fn the_form_east_handle_widens_the_control() {
    let mut harness = harness(
        app_with(ObjectKind::Form),
        datacraft_ui::form_design::form_design_ui,
    );
    let start = handle_pos("design.handle.C1.e");
    drag(&mut harness, start, start + egui::vec2(48.0, 0.0));
    assert_eq!(
        form_control(harness.state()),
        Rect {
            x: 40.0,
            y: 40.0,
            width: 208.0,
            height: 24.0,
        }
    );
}

/// The report's south-east handle grows both edges on the page.
#[test]
fn the_report_south_east_handle_grows_the_control() {
    let mut harness = harness(
        app_with(ObjectKind::Report),
        datacraft_ui::report_design::report_design_ui,
    );
    let rect = report_control(harness.state());
    let start = handle_pos("design.handle.Title.se");
    drag(&mut harness, start, start + egui::vec2(32.0, 16.0));
    assert_eq!(
        report_control(harness.state()),
        Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width + 32.0,
            height: rect.height + 16.0,
        }
    );
}

/// A corner drag that would leave the canvas is clamped by the shared
/// geometry — the dispatch is valid, so the design keeps the control.
#[test]
fn a_resize_beyond_the_canvas_is_clamped() {
    let mut harness = harness(
        app_with(ObjectKind::Form),
        datacraft_ui::form_design::form_design_ui,
    );
    let start = handle_pos("design.handle.C1.se");
    drag(&mut harness, start, start + egui::vec2(5000.0, 5000.0));
    let rect = form_control(harness.state());
    assert!(rect.x + rect.width <= 640.0, "{rect:?}");
    assert!(rect.y + rect.height <= 400.0, "{rect:?}");
}
