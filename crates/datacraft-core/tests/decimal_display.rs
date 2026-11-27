//! Scale-aware decimal display across the core consumers (phase 104,
//! step 104.1; item M135): the filter-by-value list renders the declared
//! scale, filtering by the rendered text matches, and the exports carry
//! the same rendering as the datasheet (the phase-99 workaround
//! delegates to the shared helper).

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

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
        vec![
            vec![
                FieldValue::Decimal(2550),
                FieldValue::Text("Ada".to_owned()),
            ],
            vec![
                FieldValue::Decimal(255),
                FieldValue::Text("Grace".to_owned()),
            ],
        ],
    );
    port
}

fn opened() -> Core {
    let mut core = Core::new(Box::new(rates_port()));
    core.dispatch(Command::OpenDatabase {
        path: "rates.mock".into(),
        password: None,
        source: None,
    });
    core
}

fn listed_values(events: &[Event]) -> Vec<String> {
    match events.first() {
        Some(Event::ColumnValuesListed { values, .. }) => values.clone(),
        other => panic!("expected ColumnValuesListed, got {other:?}"),
    }
}

#[test]
fn the_value_list_renders_the_declared_scale() {
    let mut core = opened();
    let events = core.dispatch(Command::ListColumnValues {
        table: "Rates".into(),
        field: "rate".into(),
    });
    assert_eq!(listed_values(&events), ["2.55", "25.50"]);
}

#[test]
fn filtering_by_the_rendered_text_matches_the_cells() {
    let mut core = opened();
    core.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Rates",
    )));
    core.dispatch(Command::FilterByValue {
        table: "Rates".into(),
        field: "rate".into(),
        value: Some("25.50".into()),
    });
    let state = core.inspect();
    let tab = state
        .tabs
        .iter()
        .find(|tab| tab.object_name.eq_ignore_ascii_case("Rates"))
        .expect("the table tab");
    assert_eq!(
        tab.value_filters.get("rate"),
        Some(&datacraft_core::state::ValueFilter::Text("25.50".into()))
    );
}

#[test]
fn the_csv_export_carries_the_same_rendering() {
    let mut core = opened();
    let path = std::env::temp_dir().join(format!(
        "datacraft-decimal-export-{}.csv",
        std::process::id()
    ));
    let events = core.dispatch(Command::ExportTable {
        table: "Rates".into(),
        format: datacraft_core::export::ExportFormat::Csv,
        path: path.clone(),
    });
    assert!(
        matches!(events.first(), Some(Event::TableExported { .. })),
        "{events:?}"
    );
    let text = std::fs::read_to_string(&path).expect("the export exists");
    let _ = std::fs::remove_file(&path);
    assert!(
        text.contains("25.50") && text.contains("2.55"),
        "the export renders the declared scale:\n{text}"
    );
    assert!(
        !text.contains("2550"),
        "the raw scaled integer must not appear:\n{text}"
    );
}
