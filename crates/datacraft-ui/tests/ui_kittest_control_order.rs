//! The designers' order surface (phase 102, step 102.3; item M133).
//!
//! The property sheet carries the selected control's z-position and tab
//! position; the four buttons dispatch exactly one order command each and
//! the canvas repaints in the new order. Core clamping and the command
//! contracts are pinned by `datacraft-core/tests/control_order.rs`.

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind, ControlProperties};
use datacraft_core::port::MockPort;
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn label(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: name.into() },
        rect: Rect {
            x: 40.0,
            y: 40.0,
            width: 120.0,
            height: 24.0,
        },
        properties: ControlProperties::default(),
    }
}

fn app_with_three() -> App {
    let mut app = App::new(Box::new(MockPort::new()));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    for name in ["a", "b", "c"] {
        app.dispatch(Command::AddFormControl {
            form: "Card".into(),
            control: label(name),
        });
    }
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Form, "Card"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_form_selection(tab, "a".into());
    app
}

type Render = fn(&mut App, &datacraft_core::snapshot::TabSnapshot, &mut eframe::egui::Ui);

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            let render: Render = datacraft_ui::form_design::form_design_ui;
            ui.set_max_width(1400.0);
            render(app, &tab, ui);
        },
        app,
    );
    harness.set_size(eframe::egui::vec2(1400.0, 1000.0));
    harness.run();
    harness
}

fn paint_order(app: &App) -> Vec<String> {
    let db = app.snapshot().database.clone().expect("open");
    db.forms[0]
        .controls
        .iter()
        .map(|control| control.name.clone())
        .collect()
}

fn tab_order(app: &App) -> Vec<String> {
    let mut entries: Vec<(u32, String)> = app.snapshot().database.clone().expect("open").forms[0]
        .controls
        .iter()
        .map(|control| (control.properties.tab_index, control.name.clone()))
        .collect();
    entries.sort();
    entries.into_iter().map(|(_, name)| name).collect()
}

/// The sheet names both positions, so the user sees where the control
/// sits before moving it.
#[test]
fn the_sheet_shows_the_control_positions() {
    let harness = harness(app_with_three());
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let text = node.value().unwrap_or_default();
        if text.contains("Z-order") || text.contains("Tab order") {
            lines.push(text);
        }
    }
    let joined = lines.join("\n");
    assert!(joined.contains("Z-order: 1 of 3"), "{joined}");
    assert!(joined.contains("Tab order: 1 of 3"), "{joined}");
}

/// Bring to Front / Send to Back move the paint order; the event's
/// position is the effective one.
#[test]
fn the_z_buttons_reorder_the_paint_order() {
    let mut harness = harness(app_with_three());
    harness.get_by_label("Bring to Front").click();
    harness.run();
    assert_eq!(paint_order(harness.state()), ["b", "c", "a"]);

    harness.get_by_label("Send to Back").click();
    harness.run();
    assert_eq!(paint_order(harness.state()), ["a", "b", "c"]);
}

/// Move Up / Move Down change the tab order only.
#[test]
fn the_tab_buttons_reorder_the_tab_order() {
    let mut harness = harness(app_with_three());
    harness.get_by_label("Move Down").click();
    harness.run();
    assert_eq!(tab_order(harness.state()), ["b", "a", "c"]);
    assert_eq!(
        paint_order(harness.state()),
        ["a", "b", "c"],
        "the paint order stays untouched"
    );

    harness.get_by_label("Move Up").click();
    harness.run();
    assert_eq!(tab_order(harness.state()), ["a", "b", "c"]);
}
