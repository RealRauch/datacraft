//! Scale-aware decimal cells in the datasheet and the form runtime
//! (phase 104, step 104.1; item M135): a scale-2 column shows `25.50`
//! where the raw scaled integer paints today.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn rates_port() -> MockPort {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Rates",
        vec![
            FieldDef::new(
                "rate",
                FieldKind::Number(NumericKind::Decimal {
                    precision: 18,
                    scale: 2,
                }),
            ),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    port.with_row_data(
        "Rates",
        vec![vec![
            FieldValue::Decimal(2550),
            FieldValue::Text("Ada".to_owned()),
        ]],
    );
    port
}

fn datasheet_app() -> App {
    let mut app = App::new(Box::new(rates_port()));
    app.dispatch(Command::OpenDatabase {
        path: "rates.mock".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Rates",
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

fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        lines.push(format!("{:?}|{label}|{value}", node.role()));
    }
    lines.join("\n")
}

#[test]
fn the_datasheet_renders_the_declared_scale() {
    let mut harness = datasheet_harness(datasheet_app());
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("25.50"),
        "the scale-2 cell renders its declared scale:\n{dump}"
    );
    assert!(
        !dump.contains("2550"),
        "the raw scaled integer must not paint:\n{dump}"
    );
}

#[test]
fn the_form_runtime_renders_the_declared_scale() {
    let mut app = datasheet_app();
    app.dispatch(Command::CreateForm { name: "Card".into() });
    app.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Rates".into(),
    });
    app.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "rate".into(),
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Form,
        "Card",
    )));
    let tab = app.snapshot().tabs.last().expect("the form tab").clone();
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::form_runtime::form_runtime_ui(app, &tab, ui);
        },
        app,
    );
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("25.50"),
        "the bound text box renders the declared scale:\n{dump}"
    );
    assert!(
        !dump.contains("2550"),
        "the raw scaled integer must not paint:\n{dump}"
    );
}
