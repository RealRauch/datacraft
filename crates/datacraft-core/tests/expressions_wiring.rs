//! Field-expression wiring contract (step 15.2, item H41).
//!
//! The expression service connects to the data paths: default values fill
//! `Null` cells on insert, validation rules run on insert and update
//! (True passes; False and Null violate), calculated fields are
//! engine-computed, expressions evaluate over the bus/MCP/CLI, and the
//! report preview renders grouped pages with aggregate bands.

use datacraft_core::command::{Command, ExpressionContext};
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::report_preview::Band;
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, FieldValue, NumericKind, TableDef,
};

fn products() -> TableDef {
    TableDef::new(
        "Products",
        vec![
            FieldDef::new(
                "id",
                FieldKind::AutoNumber(datacraft_core::schema::AutoKind::LongInteger),
            ),
            FieldDef::new("name", FieldKind::Text { max_len: 255 }),
            FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
            field_with("price", FieldKind::Currency, |p| {
                p.validation_rule = Some("[price] > 0".into());
                p.validation_text = Some("price must be positive".into());
            }),
            field_with("total", FieldKind::Currency, |p| {
                p.calculated = Some("[qty] * [price]".into())
            }),
            field_with("discount", FieldKind::Number(NumericKind::Double), |p| {
                p.default_value = Some("0.1".into())
            }),
        ],
    )
}

fn field_with(name: &str, kind: FieldKind, set: impl FnOnce(&mut FieldProperties)) -> FieldDef {
    let mut field = FieldDef::new(name, kind);
    set(&mut field.properties);
    field
}

fn seeded() -> Vec<Vec<FieldValue>> {
    vec![vec![
        FieldValue::Integer(1),
        FieldValue::Text("A".into()),
        FieldValue::Integer(2),
        FieldValue::Currency(10_000),
        FieldValue::Currency(20_000),
        FieldValue::Float(0.1),
    ]]
}

fn core_with(table: TableDef, rows: Vec<Vec<FieldValue>>) -> Core {
    let mut port = MockPort::with_tables(vec![table.clone()]);
    port.with_row_data(&table.name, rows);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    core
}

fn rows(core: &Core) -> Vec<Vec<FieldValue>> {
    core.read_rows("Products", RowRange { start: 0, len: 100 })
        .expect("read rows")
}

fn failure(events: &[Event]) -> String {
    let reason = events.iter().find_map(|event| match event {
        Event::Failed { reason, .. } => Some(reason.clone()),
        _ => None,
    });
    reason.expect("the dispatch must fail")
}

fn new_row() -> Vec<FieldValue> {
    vec![
        FieldValue::Null,
        FieldValue::Text("B".into()),
        FieldValue::Integer(3),
        FieldValue::Currency(10_000),
        FieldValue::Null,
        FieldValue::Null,
    ]
}

#[test]
fn defaults_fill_null_cells_on_insert() {
    let mut core = core_with(products(), seeded());
    let events = core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: new_row(),
    });
    assert!(matches!(events.last(), Some(Event::RowInserted { .. })));
    let last = rows(&core).pop().expect("row appended");
    assert_eq!(last[4], FieldValue::Currency(30_000)); // calculated
    assert_eq!(last[5], FieldValue::Float(0.1)); // default filled
    // An explicit value beats the default.
    let mut explicit = new_row();
    explicit[5] = FieldValue::Float(0.5);
    core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: explicit,
    });
    assert_eq!(
        rows(&core).pop().expect("row appended")[5],
        FieldValue::Float(0.5)
    );
}

#[test]
fn broken_defaults_fail_the_insert_precisely() {
    let mut table = products();
    table.fields[5].properties.default_value = Some("1 +".into());
    let mut core = core_with(table, vec![]);
    let reason = failure(&core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: new_row(),
    }));
    assert!(reason.contains("discount"), "{reason}");
    assert!(reason.contains("default"), "{reason}");
    assert!(
        rows(&core).is_empty(),
        "a rejected row never reaches storage"
    );
}

#[test]
fn validation_rules_reject_false_and_null_results() {
    let mut core = core_with(products(), seeded());
    let mut below_zero = new_row();
    below_zero[3] = FieldValue::Currency(-1);
    let reason = failure(&core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: below_zero,
    }));
    assert!(reason.contains("price must be positive"), "{reason}");

    // Null rule results violate as well (only True passes).
    let mut null_price = new_row();
    null_price[3] = FieldValue::Null;
    let reason = failure(&core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: null_price,
    }));
    assert!(reason.contains("price must be positive"), "{reason}");

    // Updates run the rule too (calculated cells stay engine-computed).
    let mut update = seeded()[0].clone();
    update[3] = FieldValue::Currency(-5);
    update[4] = FieldValue::Null;
    let reason = failure(&core.dispatch(Command::UpdateRow {
        table: "Products".into(),
        row: 0,
        values: update,
    }));
    assert!(reason.contains("price must be positive"), "{reason}");
}

#[test]
fn calculated_fields_are_engine_computed() {
    let mut core = core_with(products(), seeded());
    let mut provided = new_row();
    provided[4] = FieldValue::Currency(999);
    let reason = failure(&core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: provided,
    }));
    assert!(reason.contains("engine-computed"), "{reason}");

    core.dispatch(Command::UpdateRow {
        table: "Products".into(),
        row: 0,
        values: vec![
            FieldValue::Null,
            FieldValue::Text("A".into()),
            FieldValue::Integer(5),
            FieldValue::Currency(20_000),
            FieldValue::Null,
            FieldValue::Float(0.1),
        ],
    });
    assert_eq!(
        rows(&core)[0][4],
        FieldValue::Currency(100_000),
        "updates recompute calculated fields"
    );
}

#[test]
fn calculated_results_validate_against_the_field_kind() {
    let mut table = products();
    table.fields[4].properties.calculated = Some("[qty]".into()); // Integer into Currency
    let mut core = core_with(table, vec![]);
    let reason = failure(&core.dispatch(Command::InsertRow {
        table: "Products".into(),
        values: new_row(),
    }));
    assert!(reason.contains("total"), "{reason}");
}

#[test]
fn run_expression_evaluates_over_the_bus() {
    let mut core = core_with(products(), seeded());
    let events = core.dispatch(Command::RunExpression {
        context: None,
        expression: "1+2".into(),
    });
    assert!(matches!(
        events.last(),
        Some(Event::ExpressionEvaluated { result }) if result == "3"
    ));
    assert_eq!(core.expression_result(), Some(&FieldValue::Integer(3)));

    let events = core.dispatch(Command::RunExpression {
        context: Some(ExpressionContext {
            table: "products".into(),
            row: 0,
        }),
        expression: "[qty] * [price]".into(),
    });
    assert!(matches!(
        events.last(),
        Some(Event::ExpressionEvaluated { result }) if result == "2"
    ));

    for (expression, needle) in [("Ghost", "Ghost"), ("1 +", "parse error")] {
        let reason = failure(&core.dispatch(Command::RunExpression {
            context: Some(ExpressionContext {
                table: "Products".into(),
                row: 0,
            }),
            expression: expression.into(),
        }));
        assert!(reason.contains(needle), "{expression:?}: {reason}");
        assert!(core.expression_result().is_none());
    }
}

fn report_with_bands(core: &mut Core) {
    for command in [
        Command::CreateReport {
            name: "Invoice".into(),
        },
        Command::SetReportSource {
            report: "Invoice".into(),
            table: "Products".into(),
        },
        Command::AddReportControl {
            report: "Invoice".into(),
            control: datacraft_core::form::ControlDef {
                name: "qtyBox".into(),
                kind: datacraft_core::form::ControlKind::TextBox {
                    bound_field: Some("qty".into()),
                },
                rect: datacraft_core::canvas::Rect {
                    x: 36.0,
                    y: 60.0,
                    width: 100.0,
                    height: 20.0,
                },
            },
        },
        Command::AddReportControl {
            report: "Invoice".into(),
            control: datacraft_core::form::ControlDef {
                name: "sumBox".into(),
                kind: datacraft_core::form::ControlKind::TextBox {
                    bound_field: Some("=Sum([price])".into()),
                },
                rect: datacraft_core::canvas::Rect {
                    x: 36.0,
                    y: 700.0,
                    width: 100.0,
                    height: 20.0,
                },
            },
        },
    ] {
        let events = core.dispatch(command);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. })),
            "report setup must succeed"
        );
    }
}

fn banded_rows() -> Vec<Vec<FieldValue>> {
    let row = |id: i64, name: &str, qty: i64, price: i64| {
        vec![
            FieldValue::Integer(id),
            FieldValue::Text(name.into()),
            FieldValue::Integer(qty),
            FieldValue::Currency(price),
            FieldValue::Null,
            FieldValue::Null,
        ]
    };
    vec![
        row(1, "A", 2, 10_000),
        row(2, "A", 1, 20_000),
        row(3, "B", 4, 5_000),
    ]
}

#[test]
fn report_bands_group_and_aggregate() {
    let mut core = core_with(products(), banded_rows());
    report_with_bands(&mut core);
    let events = core.dispatch(Command::SetReportGrouping {
        report: "Invoice".into(),
        group_by: Some("name".into()),
    });
    assert!(matches!(
        events.last(),
        Some(Event::ReportGroupingSet { group_by: Some(group), .. }) if group == "name"
    ));

    let pages = core.report_preview("Invoice").expect("preview renders");
    let bands: Vec<(Band, Vec<&str>)> = pages
        .iter()
        .map(|page| {
            (
                page.band,
                page.items.iter().map(|item| item.text.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        bands,
        vec![
            (Band::GroupHeader, vec!["A"]),
            (Band::Detail, vec!["2", "1"]), // qty, Sum over the record's group member
            (Band::Detail, vec!["1", "2"]),
            (Band::GroupFooter, vec!["3"]), // Sum of the group's prices
            (Band::GroupHeader, vec!["B"]),
            (Band::Detail, vec!["4", "0.5"]),
            (Band::GroupFooter, vec!["0.5"]),
        ]
    );
}

#[test]
fn ungrouped_previews_stay_single_record_pages() {
    let mut core = core_with(products(), banded_rows());
    report_with_bands(&mut core);
    core.dispatch(Command::SetReportGrouping {
        report: "Invoice".into(),
        group_by: None,
    });
    let pages = core.report_preview("Invoice").expect("preview renders");
    assert!(pages.iter().all(|page| page.band == Band::Detail));
    assert_eq!(pages.len(), 3);
    assert_eq!(
        pages[0].items[1].text, "1",
        "aggregates see the record's row"
    );
}

#[test]
fn grouping_whitelists_against_the_source() {
    let mut core = core_with(products(), seeded());
    core.dispatch(Command::CreateReport {
        name: "Invoice".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Invoice".into(),
        table: "Products".into(),
    });
    let reason = failure(&core.dispatch(Command::SetReportGrouping {
        report: "Invoice".into(),
        group_by: Some("Ghost".into()),
    }));
    assert!(reason.contains("Ghost"), "{reason}");
}

#[test]
fn broken_band_expressions_fail_the_preview() {
    let mut core = core_with(products(), banded_rows());
    report_with_bands(&mut core);
    core.dispatch(Command::AddReportControl {
        report: "Invoice".into(),
        control: datacraft_core::form::ControlDef {
            name: "brokenBox".into(),
            kind: datacraft_core::form::ControlKind::TextBox {
                bound_field: Some("=Sum([ghost])".into()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 36.0,
                y: 720.0,
                width: 100.0,
                height: 20.0,
            },
        },
    });
    let error = core
        .report_preview("Invoice")
        .expect_err("unknown fields must fail");
    assert!(error.contains("ghost"), "{error}");
}
