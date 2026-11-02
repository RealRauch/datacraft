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

#[test]
fn unknown_source_tables_are_precise_in_run() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let native = query(&["Ghost"], vec![column("Ghost.*")]);
    let error = run_with_limit(&native, &sources, 100).expect_err("the table is missing");
    assert_eq!(
        error,
        NativeExecError::UnknownTable {
            table: "Ghost".to_owned()
        }
    );
}

#[test]
fn a_second_inner_join_filters_already_bound_tables() {
    let a = table("A", &["id"]);
    let b = table("B", &["id"]);
    let c = table("C", &["id"]);
    let sources = vec![
        (&a, rows(&[1, 2])),
        (&b, rows(&[2, 3])),
        (&c, rows(&[2, 4])),
    ];
    let mut native = query(&["A", "B", "C"], vec![column("A.id")]);
    for (expression, kind) in [
        ("A.id = B.id", NativeJoinKind::Inner),
        ("B.id = C.id", NativeJoinKind::Inner),
        ("A.id = C.id", NativeJoinKind::Inner),
    ] {
        native.joins.push(NativeJoin {
            expression: expression.to_owned(),
            kind,
        });
    }
    let result = run_with_limit(&native, &sources, 100).expect("the three joins run");
    assert_eq!(result.rows, vec![vec![FieldValue::Integer(2)]]);
}

#[test]
fn a_left_join_preserves_a_new_left_operand() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1, 2])), (&b, rows(&[2, 3]))];
    let mut native = query(&["A", "B"], vec![column("A.id"), column("B.id")]);
    native.joins.push(NativeJoin {
        expression: "B.id = A.id".to_owned(),
        kind: NativeJoinKind::Left,
    });
    let result = run_with_limit(&native, &sources, 100).expect("the left join runs");
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Integer(2), FieldValue::Integer(2)],
            vec![FieldValue::Null, FieldValue::Integer(3)],
        ]
    );
}

#[test]
fn a_right_join_preserves_a_bound_right_operand() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1, 2])), (&b, rows(&[2, 3]))];
    let mut native = query(&["A", "B"], vec![column("A.id"), column("B.id")]);
    native.joins.push(NativeJoin {
        expression: "B.id = A.id".to_owned(),
        kind: NativeJoinKind::Right,
    });
    let result = run_with_limit(&native, &sources, 100).expect("the right join runs");
    assert_eq!(
        result.rows,
        vec![
            vec![FieldValue::Integer(1), FieldValue::Null],
            vec![FieldValue::Integer(2), FieldValue::Integer(2)],
        ]
    );
}

#[test]
fn a_query_without_projected_columns_is_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let native = query(&["A"], Vec::new());
    let error = run_with_limit(&native, &sources, 100).expect_err("no projection");
    assert!(error.to_string().contains("projects no columns"), "{error}");
}

#[test]
fn a_query_without_source_tables_is_precise() {
    let native = query(&[], vec![column("A.id")]);
    let error = run_with_limit(&native, &[], 100).expect_err("no FROM");
    assert!(error.to_string().contains("no source table"), "{error}");
}

#[test]
fn kind_labels_cover_every_flavor() {
    use super::kind_label;
    for (kind, label) in [
        (NativeQueryKind::Select, "select"),
        (NativeQueryKind::Crosstab, "crosstab"),
        (NativeQueryKind::Delete, "delete"),
        (NativeQueryKind::Update, "update"),
        (NativeQueryKind::Append, "append"),
        (NativeQueryKind::MakeTable, "make-table"),
        (NativeQueryKind::DataDefinition, "data-definition"),
        (NativeQueryKind::Passthrough, "pass-through"),
        (NativeQueryKind::Union, "union"),
    ] {
        assert_eq!(kind_label(kind), label);
    }
}

#[test]
fn unknown_qualified_tables_in_columns_are_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let native = query(&["A"], vec![column("Ghost.id")]);
    let error = run_with_limit(&native, &sources, 100).expect_err("the table is missing");
    assert!(
        matches!(error, NativeExecError::UnknownTable { .. }),
        "{error}"
    );
}

#[test]
fn unknown_bare_fields_are_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let mut native = query(&["A"], vec![column("A.id")]);
    native.criteria.push("ghost > 1".to_owned());
    let error = run_with_limit(&native, &sources, 100).expect_err("the field is missing");
    assert!(
        matches!(error, NativeExecError::UnknownField { table: None, .. }),
        "{error}"
    );
}

#[test]
fn boolean_expression_shapes_validate_and_run() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1, 2, 3]))];
    let mut native = query(&["A"], vec![column("A.id")]);
    native
        .criteria
        .push("Not (A.id = 2) And A.id Between 1 And 3 And A.id In (1, 3)".to_owned());
    let result = run_with_limit(&native, &sources, 100).expect("the expression runs");
    assert_eq!(
        result.rows,
        vec![vec![FieldValue::Integer(1)], vec![FieldValue::Integer(3)],]
    );
}

#[test]
fn a_column_without_a_reference_is_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let mut native = query(&["A"], Vec::new());
    native.columns.push(NativeQueryColumn {
        table: None,
        field: None,
        expression: None,
        descending: false,
    });
    let error = run_with_limit(&native, &sources, 100).expect_err("no reference");
    assert!(
        error.to_string().contains("without a field reference"),
        "{error}"
    );
}

#[test]
fn a_wildcard_of_an_unknown_table_is_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let native = query(&["A"], vec![column("Ghost.*")]);
    let error = run_with_limit(&native, &sources, 100).expect_err("the table is missing");
    assert!(
        matches!(error, NativeExecError::UnknownTable { .. }),
        "{error}"
    );
}

#[test]
fn a_bare_star_after_a_dot_expands_every_table() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1, 2])), (&b, rows(&[3, 4]))];
    let native = query(&["A", "B"], vec![column(".*")]);
    let result = run_with_limit(&native, &sources, 100).expect("the wildcard runs");
    assert_eq!(result.columns, ["id", "id"]);
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn filter_evaluation_errors_are_precise() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let mut native = query(&["A"], vec![column("A.id")]);
    native.criteria.push("Nope(A.id) = 1".to_owned());
    let error = run_with_limit(&native, &sources, 100).expect_err("the function is unknown");
    assert!(
        matches!(error, NativeExecError::BadExpression { .. }),
        "{error}"
    );
}

#[test]
fn order_by_must_be_a_plain_field() {
    let (a, _) = two_tables();
    let sources = vec![(&a, rows(&[1]))];
    let mut native = query(&["A"], vec![column("A.id")]);
    native.order_by.push(crate::port::NativeOrder {
        field: "A.id + 1".to_owned(),
        descending: false,
    });
    let error = run_with_limit(&native, &sources, 100).expect_err("computed order is refused");
    assert!(error.to_string().contains("computed"), "{error}");
}

#[test]
fn join_parse_errors_are_locatable() {
    let (a, b) = two_tables();
    let sources = vec![(&a, rows(&[1])), (&b, rows(&[1]))];
    for (expression, fragment) in [
        ("A.id =", "expression"),
        ("A.id + 1 = B.id", "plain field references"),
        ("Ghost.id = B.id", "Ghost"),
        ("A.ghost = B.id", "ghost"),
    ] {
        let mut native = query(&["A", "B"], vec![column("A.id")]);
        native.joins.push(NativeJoin {
            expression: expression.to_owned(),
            kind: NativeJoinKind::Inner,
        });
        let error = run_with_limit(&native, &sources, 100).expect_err("the join is invalid");
        assert!(
            error.to_string().contains(fragment),
            "{expression}: {error}"
        );
    }
}
