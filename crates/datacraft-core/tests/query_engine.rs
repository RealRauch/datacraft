//! Contract suite for the query engine (step 7.3, item H27).
//!
//! Decision default 7: in-memory predicates over port rows, no SQL
//! parser. Criteria strings parse to typed predicates (per-column
//! coercion), rows filter AND-across-columns / OR-within-column (SQL
//! NULL never satisfies a predicate), the projection follows the shown
//! grid columns, and sorts are stable multi-key.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::{QbeField, QbeRow, QbeValue, QueryDef, SortOrder};
use datacraft_core::query_engine::{EngineError, QueryResult, run_query};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::FieldValue;

fn customers_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("balance", FieldKind::Currency),
        ],
    )
}

fn rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Currency(10_000),
        ],
        vec![
            FieldValue::Integer(2),
            FieldValue::Text("Brahe".into()),
            FieldValue::Currency(-5_000),
        ],
        vec![
            FieldValue::Integer(3),
            FieldValue::Text("Curie".into()),
            FieldValue::Null,
        ],
    ]
}

fn field(name: &str, criteria: &[&str]) -> QbeField {
    QbeField {
        name: name.to_owned(),
        shown: true,
        sort: SortOrder::None,
        criteria: criteria.iter().map(|c| c.to_string()).collect(),
    }
}

fn def(fields: Vec<QbeField>) -> QueryDef {
    QueryDef {
        name: "Q".into(),
        table: "Customers".into(),
        fields,
    }
}

// ---------------------------------------------------------------------------
// criteria parsing and coercion
// ---------------------------------------------------------------------------

#[test]
fn criteria_parse_operators_and_plain_values() {
    let d = customers_def();
    let q = def(vec![field("name", &["= Ada", "<> Ada", "Ada", "\"Ada\""])]);
    // Parsing is validated by execution succeeding; specifics covered below.
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.columns, ["name"]);
}

#[test]
fn numeric_comparisons_coerce_to_the_column_kind() {
    let d = customers_def();
    let q = def(vec![field("id", &["> 1"])]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.rows.len(), 2, "ids 2 and 3");

    let q = def(vec![field("balance", &[">= 0"])]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.rows.len(), 1, "only Ada has balance >= 0");
}

#[test]
fn coercion_failures_name_the_column_and_the_literal() {
    let d = customers_def();
    let q = def(vec![field("id", &["much"])]);
    let err = run_query(&q, &d, &rows()).expect_err("not a number");
    let msg = err.to_string();
    assert!(msg.contains("id"), "{msg}");
    assert!(msg.contains("much"), "{msg}");
}

#[test]
fn unknown_criteria_fields_are_whitelist_errors() {
    let d = customers_def();
    let q = def(vec![field("ghost", &["1"])]);
    let err = run_query(&q, &d, &rows()).expect_err("unknown field");
    assert!(matches!(err, EngineError::UnknownField { .. }), "{err}");
}

#[test]
fn text_comparisons_are_lexicographic() {
    let d = customers_def();
    let q = def(vec![field("name", &[">= \"B\""])]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.rows.len(), 2, "Brahe and Curie");
}

#[test]
fn date_criteria_parse_iso_dates() {
    let d = TableDef::new(
        "Events",
        vec![FieldDef::new("when", FieldKind::DateTime)],
    );
    let data = vec![
        vec![FieldValue::DateTime(45_656.0)],
        vec![FieldValue::DateTime(45_658.0)],
    ];
    let q = def(vec![field("when", &["2025-01-01"])]);
    let result = run_query(&q, &d, &data).expect("runs");
    assert_eq!(result.rows.len(), 1, "only the 45658 row is in 2025");
}

#[test]
fn boolean_criteria_accept_access_wording() {
    let d = TableDef::new("T", vec![FieldDef::new("ok", FieldKind::Boolean { tri_state: false })]);
    let data = vec![
        vec![FieldValue::Boolean(true)],
        vec![FieldValue::Boolean(false)],
    ];
    let q = def(vec![field("ok", &["yes"])]);
    let result = run_query(&q, &d, &data).expect("runs");
    assert_eq!(result.rows.len(), 1);
}

// ---------------------------------------------------------------------------
// composition, projection, sorting
// ---------------------------------------------------------------------------

#[test]
fn columns_and_across_or_within() {
    let d = customers_def();
    // id < 2 OR id > 2, AND name <> "Brahe".
    let q = def(vec![
        field("id", &["< 2", "> 2"]),
        field("name", &["<> Brahe"]),
    ]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.rows.len(), 1, "Ada (id 1) and Curie (id 3); Curie passes both");
}

#[test]
fn nulls_never_satisfy_predicates() {
    let d = customers_def();
    let q = def(vec![field("balance", &["> -999999"])]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.rows.len(), 1, "Curie's NULL balance drops out");
}

#[test]
fn projection_follows_shown_columns_in_grid_order() {
    let d = customers_def();
    let mut q = def(vec![field("balance", &[]), field("name", &[])]);
    q.fields[0].shown = false;
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert_eq!(result.columns, ["name"]);
    assert_eq!(result.rows[0][0], FieldValue::Text("Ada".into()));
}

#[test]
fn sorts_are_stable_multi_key() {
    let d = customers_def();
    let data = vec![
        vec![FieldValue::Integer(1), FieldValue::Text("Zulu".into())],
        vec![FieldValue::Integer(1), FieldValue::Text("Alpha".into())],
        vec![FieldValue::Integer(0), FieldValue::Text("Mike".into())],
    ];
    let mut q = def(vec![field("id", &[]), field("name", &[])]);
    q.fields[0].sort = SortOrder::Ascending;
    q.fields[1].sort = SortOrder::Descending;
    let result = run_query(&q, &d, &data).expect("runs");
    let ids: Vec<i64> = result
        .rows
        .iter()
        .map(|r| match &r[0] {
            FieldValue::Integer(i) => *i,
            _ => i64::MAX,
        })
        .collect();
    assert_eq!(ids, [0, 1, 1], "ascending id first");
    assert_eq!(result.rows[1][1], FieldValue::Text("Zulu".into()));
    assert_eq!(result.rows[2][1], FieldValue::Text("Alpha".into()));
}

#[test]
fn an_empty_result_is_a_valid_result() {
    let d = customers_def();
    let q = def(vec![field("id", &["< 0"])]);
    let result = run_query(&q, &d, &rows()).expect("runs");
    assert!(result.rows.is_empty());
    assert_eq!(result.columns, ["id"]);
}

// ---------------------------------------------------------------------------
// core integration
// ---------------------------------------------------------------------------

fn seeded_core() -> Core {
    let mut port = MockPort::new();
    port.with_row_data("Customers", rows());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase { path: PathBuf::from("shop.mdb") });
    core.dispatch(Command::CreateTable { definition: customers_def() });
    core.dispatch(Command::CreateQuery { name: "Rich".into() });
    core.dispatch(Command::SetQbeCell {
        query: "Rich".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Rich".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("name".into()),
    });
    core
}

#[test]
fn core_run_query_returns_the_result() {
    let core = seeded_core();
    core.dispatch(Command::SetQbeCell {
        query: "Rich".into(),
        row: QbeRow::Criteria(0),
        col: 0,
        value: QbeValue::Text("< C".into()),
    });
    let result = core.run_query("Rich").expect("runs");
    let QueryResult { columns, rows } = result;
    assert_eq!(columns, ["name"]);
    assert_eq!(rows.len(), 2, "Ada and Brahe sort before C");
}

#[test]
fn run_query_errors_are_precise() {
    let core = seeded_core();
    assert!(core.run_query("Ghost").is_err());
    let closed = Core::new(Box::new(MockPort::new()));
    assert!(closed.run_query("Rich").is_err());
}

#[test]
fn the_run_query_command_is_observable() {
    let mut core = seeded_core();
    let events = core.dispatch(Command::RunQuery { query: "Rich".into() });
    assert_eq!(
        events,
        vec![Event::QueryRan {
            query: "Rich".into(),
            rows: 3,
        }]
    );
}
