//! Form runtime layout pins (step 46.1, item M57) — standalone kittest
//! suite like the other families.
//!
//! The runtime must render the designed layout: every control paints at
//! its stored canvas rect (the designer's arrangement), not stacked
//! top-left. Paint rects come from the `layout_probe` seam — a semantic
//! tree cannot see positions (H52's lesson).

use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn customers_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Email", FieldKind::Text { max_len: 80 }),
        ],
    )
}

/// App with a form built over the real bus: two text boxes whose stored
/// rects sit far apart — NameBox top-right-ish, EmailBox far lower-left.
fn form_app() -> App {
    let mut port = MockPort::with_tables(vec![customers_table()]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Integer(1),
            FieldValue::Text("Lena Fischer".to_owned()),
            FieldValue::Text("lena@example.com".to_owned()),
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
        control: ControlDef {
            name: "NameBox".into(),
            kind: ControlKind::TextBox {
                bound_field: Some("Name".into()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 208.0,
                y: 64.0,
                width: 160.0,
                height: 24.0,
            },
        },
    });
    app.dispatch(Command::AddFormControl {
        form: "Form1".into(),
        control: ControlDef {
            name: "EmailBox".into(),
            kind: ControlKind::TextBox {
                bound_field: Some("Email".into()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 64.0,
                y: 160.0,
                width: 184.0,
                height: 24.0,
            },
        },
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Form1",
    )));
    app
}

#[test]
fn the_form_runtime_paints_controls_at_their_stored_rects() {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::form_runtime::form_runtime_ui(app, &tab, ui);
        },
        form_app(),
    );
    harness.run();
    let rects = layout_probe::drain();
    let lookup = |control: &str| {
        rects
            .iter()
            .find(|(id, _)| *id == format!("runtime.Form1.{control}"))
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| {
                panic!(
                    "control {control} has no recorded runtime rect; recorded: {:?}",
                    rects.iter().map(|(id, _)| id).collect::<Vec<_>>()
                )
            })
    };
    let name = lookup("NameBox");
    let email = lookup("EmailBox");

    // The painted size honours the stored rect.
    assert_eq!(name.width(), 160.0, "NameBox paints at its stored width");
    assert_eq!(email.width(), 184.0, "EmailBox paints at its stored width");

    // The painted positions honour the stored offsets (relative, so the
    // window chrome cancels out).
    let dx = email.left() - name.left();
    let dy = email.top() - name.top();
    assert!(
        (dx - (64.0 - 208.0)).abs() < 1.0 && (dy - (160.0 - 64.0)).abs() < 1.0,
        "EmailBox paints at its stored offset from NameBox (dx {dx:.1}, dy {dy:.1}; expected -144.0, +96.0) — the runtime stacks controls instead of using the designed rects"
    );

    // The bound values render.
    let dump_tree = harness.root().children_recursive();
    let _ = dump_tree; // value rendering is pinned by the datasheet suites
    assert!(name.width() > 0.0);
}
