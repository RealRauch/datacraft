//! SQL engine contract (step 13.3, item H37).
//!
//! The engine materializes the loaded tables into a throwaway in-memory
//! SQLite database and executes exactly one `SELECT`/`WITH` statement
//! against it. Sources are never touched; the gate rejects every other
//! statement before SQLite sees it.

use datacraft_core::schema::{
    AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_core::sql_engine::{SqlResult, run_sql};

fn customers() -> TableDef {
    TableDef::new(
        "customers",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

fn orders() -> TableDef {
    TableDef::new(
        "orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("total", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

fn sources() -> Vec<(TableDef, Vec<Vec<FieldValue>>)> {
    vec![
        (
            customers(),
            vec![
                vec![
                    FieldValue::Integer(1),
                    FieldValue::Text("Ada".into()),
                    FieldValue::Boolean(true),
                ],
                vec![
                    FieldValue::Integer(2),
                    FieldValue::Text("Grace".into()),
                    FieldValue::Boolean(false),
                ],
            ],
        ),
        (
            orders(),
            vec![
                vec![
                    FieldValue::Integer(10),
                    FieldValue::Integer(1),
                    FieldValue::Float(19.5),
                ],
                vec![
                    FieldValue::Integer(11),
                    FieldValue::Integer(1),
                    FieldValue::Float(4.25),
                ],
                vec![
                    FieldValue::Integer(12),
                    FieldValue::Integer(2),
                    FieldValue::Float(7.0),
                ],
            ],
        ),
    ]
}

#[test]
fn selects_literals_and_names_the_result_columns() {
    let result = run_sql("SELECT 1 AS one, 'x' AS name, NULL AS missing", &[]).expect("run SQL");
    assert_eq!(result.columns, ["one", "name", "missing"]);
    assert_eq!(
        result.rows,
        vec![vec![
            FieldValue::Integer(1),
            FieldValue::Text("x".into()),
            FieldValue::Null
        ]]
    );
}

#[test]
fn selects_filtered_rows_in_the_requested_order() {
    let result = run_sql(
        "SELECT name FROM customers WHERE active = 1 ORDER BY name",
        &sources(),
    )
    .expect("run SQL");
    assert_eq!(result.columns, ["name"]);
    assert_eq!(result.rows, vec![vec![FieldValue::Text("Ada".into())]]);
}

#[test]
fn aggregates_and_joins_run_over_the_materialized_sources() {
    let result = run_sql(
        "SELECT c.name, COUNT(o.id) AS orders, SUM(o.total) AS total
         FROM customers c JOIN orders o ON o.customer_id = c.id
         GROUP BY c.name ORDER BY c.name",
        &sources(),
    )
    .expect("run SQL");
    assert_eq!(result.columns, ["name", "orders", "total"]);
    assert_eq!(
        result.rows,
        vec![
            vec![
                FieldValue::Text("Ada".into()),
                FieldValue::Integer(2),
                FieldValue::Float(23.75)
            ],
            vec![
                FieldValue::Text("Grace".into()),
                FieldValue::Integer(1),
                FieldValue::Float(7.0)
            ],
        ]
    );
}

#[test]
fn with_statements_are_allowed() {
    let result = run_sql(
        "WITH active AS (SELECT id FROM customers WHERE active = 1) \
         SELECT COUNT(*) AS n FROM active",
        &sources(),
    )
    .expect("run SQL");
    assert_eq!(result.rows, vec![vec![FieldValue::Integer(1)]]);
}

#[test]
fn a_trailing_semicolon_is_accepted() {
    let result = run_sql("SELECT 1 AS one;  ", &[]).expect("trailing semicolon is fine");
    assert_eq!(result.rows.len(), 1);
}

#[test]
fn only_select_and_with_statements_pass_the_gate() {
    for sql in [
        "INSERT INTO customers VALUES (3, 'x', 0)",
        "UPDATE customers SET name = 'x'",
        "DELETE FROM customers",
        "DROP TABLE customers",
        "PRAGMA table_info(customers)",
        "CREATE TABLE t (x)",
    ] {
        let err = run_sql(sql, &sources()).expect_err("write/DDL statements must be rejected");
        assert!(
            err.contains("only SELECT and WITH"),
            "precise gate reason for {sql:?}: {err}"
        );
    }
}

#[test]
fn multiple_statements_are_rejected_precisely() {
    let err = run_sql("SELECT 1; SELECT 2", &[]).expect_err("two statements must be rejected");
    assert!(err.contains("one statement"), "{err}");
    let err = run_sql("SELECT 1; DROP TABLE customers", &sources())
        .expect_err("statement smuggling must be rejected");
    assert!(err.contains("one statement"), "{err}");
}

#[test]
fn empty_input_is_rejected() {
    for sql in ["", "   ", ";\n"] {
        let err = run_sql(sql, &[]).expect_err("empty SQL must fail");
        assert!(!err.is_empty(), "reason for {sql:?}");
    }
}

#[test]
fn an_unknown_table_surfaces_the_engine_error_without_panicking() {
    let err = run_sql("SELECT * FROM missing", &sources()).expect_err("unknown table fails");
    assert!(err.contains("missing"), "{err}");
}

#[test]
fn sources_are_not_touched_and_each_call_is_isolated() {
    let mut data = sources();
    let before = data.clone();
    let _ = run_sql("SELECT * FROM customers", &data).expect("read");
    assert_eq!(data, before, "materialization must not mutate the sources");

    let result = run_sql("SELECT COUNT(*) AS n FROM customers", &data).expect("second run");
    assert_eq!(result.rows, vec![vec![FieldValue::Integer(2)]]);
}

#[test]
fn sql_result_serializes_for_transports() {
    let result = SqlResult {
        columns: vec!["n".into()],
        rows: vec![vec![FieldValue::Integer(1)]],
    };
    let json = serde_json::to_string(&result).expect("serialize result");
    let back: SqlResult = serde_json::from_str(&json).expect("deserialize result");
    assert_eq!(back, result);
}
