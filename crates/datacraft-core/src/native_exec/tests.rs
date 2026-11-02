//! Unit contracts of the native execution internals (step 27.1, M12):
//! the row cap, unbound-table handling, the wildcard parser and every
//! error display arm.

use crate::port::{NativeJoin, NativeJoinKind, NativeQuery, NativeQueryColumn, NativeQueryKind};
use crate::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

use super::{NativeExecError, run_with_limit};

fn number(name: &str) -> FieldDef {
    FieldDef::new(name, FieldKind::Number(NumericKind::LongInteger))
}

fn table(name: &str, fields: &[&str]) -> TableDef {
    TableDef::new(name, fields.iter().map(|field| number(field)).collect())
}

fn column(expression: &str) -> NativeQueryColumn {
    NativeQueryColumn {
        table: None,
        field: None,
        expression: Some(expression.to_owned()),
        descending: false,
    }
}

fn query(tables: &[&str], columns: Vec<NativeQueryColumn>) -> NativeQuery {
    NativeQuery {
        name: "Q".to_owned(),
        kind: NativeQueryKind::Select,
        tables: tables.iter().map(|table| (*table).to_owned()).collect(),
        columns,
        criteria: Vec::new(),
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }
}

fn two_tables() -> (TableDef, TableDef) {
    (table("A", &["id"]), table("B", &["id"]))
}

fn rows(values: &[i64]) -> Vec<Vec<FieldValue>> {
    values
        .iter()
        .map(|value| vec![FieldValue::Integer(*value)])
        .collect()
}

#[test]
fn the_row_cap_is_precise() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1, 2])), (&b, rows(&[3, 4]))];
    let native = query(&["A", "B"], vec![column("A.*"), column("B.*")]);
    let error = run_with_limit(&native, &sources, 3).expect_err("4 joined rows exceed 3");
    assert_eq!(error, NativeExecError::TooManyRows { rows: 4, limit: 3 });
    assert!(error.to_string().contains('4'), "{error}");
    assert!(error.to_string().contains('3'), "{error}");
}

#[test]
fn a_bare_star_cross_joins_unbound_tables() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1, 2])), (&b, rows(&[3, 4]))];
    let native = query(&["A", "B"], vec![column("*")]);
    let result = run_with_limit(&native, &sources, 100).expect("the cartesian runs");
    assert_eq!(result.columns, ["id", "id"]);
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn a_right_join_with_no_bound_rows_keeps_every_new_row() {
    let (a, b) = two_tables();
    let sources = vec![(&a, Vec::new()), (&b, rows(&[3, 4]))];
    let mut native = query(&["A", "B"], vec![column("A.id"), column("B.id")]);
    native.joins.push(NativeJoin {
        expression: "A.id = B.id".to_owned(),
        kind: NativeJoinKind::Right,
    });
    let result = run_with_limit(&native, &sources, 100).expect("the right join runs");
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Null, FieldValue::Integer(3)],
            vec![FieldValue::Null, FieldValue::Integer(4)],
        ]
    );
}

#[test]
fn a_second_outer_join_on_bound_tables_is_refused() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1])), (&b, rows(&[1]))];
    let mut native = query(&["A", "B"], vec![column("A.id")]);
    native.joins.push(NativeJoin {
        expression: "A.id = B.id".to_owned(),
        kind: NativeJoinKind::Left,
    });
    native.joins.push(NativeJoin {
        expression: "A.id = B.id".to_owned(),
        kind: NativeJoinKind::Left,
    });
    let error = run_with_limit(&native, &sources, 100).expect_err("outer join stays unsupported");
    assert!(error.to_string().contains("outer join"), "{error}");
}

#[test]
fn an_unconnected_join_is_refused() {
    let a = table("A", &["id"]);
    let b = table("B", &["id"]);
    let c = table("C", &["id"]);
    let sources = vec![(&a, rows(&[1])), (&b, rows(&[1])), (&c, rows(&[1]))];
    let mut native = query(&["A", "B", "C"], vec![column("A.id")]);
    native.joins.push(NativeJoin {
        expression: "B.id = C.id".to_owned(),
        kind: NativeJoinKind::Inner,
    });
    let error = run_with_limit(&native, &sources, 100).expect_err("the join never connects");
    assert!(error.to_string().contains("does not connect"), "{error}");
}

#[test]
fn bad_expressions_name_their_source() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let mut native = query(&["A"], vec![column("A.id")]);
    native.criteria.push("A.id >".to_owned());
    let error = run_with_limit(&native, &sources, 100).expect_err("the criterion is malformed");
    let NativeExecError::BadExpression { source, .. } = error else {
        panic!("expected BadExpression, got {error:?}");
    };
    assert_eq!(source, "A.id >");
}

#[test]
fn unknown_qualified_fields_are_named() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let native = query(&["A"], vec![column("A.ghost")]);
    let error = run_with_limit(&native, &sources, 100).expect_err("the field does not exist");
    assert_eq!(
        error,
        NativeExecError::UnknownField {
            table: Some("A".to_owned()),
            field: "ghost".to_owned(),
        }
    );
    assert!(error.to_string().contains("ghost"), "{error}");
}

#[test]
fn error_displays_cover_every_variant() {
    let variants = [
        NativeExecError::UnknownTable {
            table: "T".to_owned(),
        },
        NativeExecError::UnknownField {
            table: None,
            field: "x".to_owned(),
        },
        NativeExecError::AmbiguousField {
            field: "x".to_owned(),
        },
        NativeExecError::BadExpression {
            source: "x ?".to_owned(),
            detail: "parse error".to_owned(),
        },
        NativeExecError::TooManyRows { rows: 9, limit: 1 },
    ];
    for variant in variants {
        assert!(!variant.to_string().is_empty(), "{variant:?}");
    }
}
