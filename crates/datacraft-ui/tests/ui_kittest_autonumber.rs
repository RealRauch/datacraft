//! The AutoNumber new-record gate (step 91.6, item M114): engine-assigned
//! columns render disabled before input. The gate's rect rides the
//! layout probe under `datasheet.newrow.autonumber_gate` — the trailing
//! row sits below the a11y fold in headless harnesses, so presence/absence
//! of the probe is the honest seam. The hint strings are pinned in the
//! crate's unit tests (`trailing.rs`).

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use egui_kittest::Harness;

fn app_with_table(autonumber: bool) -> App {
    let fields = if autonumber {
        vec![
            FieldDef::new("Id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ]
    } else {
        vec![
            FieldDef::new(
                "Id",
                FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
            ),
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
        ]
    };
    let table = TableDef::new("Customers", fields);
    let mut port = MockPort::with_tables(vec![table]);
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
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    app
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

/// With an AutoNumber column the gate renders; a plain number column
/// keeps the editable cell and paints no gate.
#[test]
fn the_autonumber_cell_gates_and_a_plain_cell_does_not() {
    let mut harness = datasheet_harness(app_with_table(true));
    harness.run();
    assert!(
        layout_probe::drain()
            .iter()
            .any(|(id, _)| id == "datasheet.newrow.autonumber_gate"),
        "the AutoNumber cell renders its gate"
    );

    let mut harness = datasheet_harness(app_with_table(false));
    harness.run();
    assert!(
        !layout_probe::drain()
            .iter()
            .any(|(id, _)| id == "datasheet.newrow.autonumber_gate"),
        "a plain cell renders no gate"
    );
}
