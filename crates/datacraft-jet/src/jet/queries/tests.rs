//! Decoder contracts for MSysQueries attributes (step 27.1, item M12):
//! the typed join/order/parameter fields and every precise rejection.

use super::*;
use crate::types::StorageValue;

fn row(
    attribute: Option<i64>,
    name1: Option<&str>,
    name2: Option<&str>,
    expression: Option<&str>,
    flag: Option<i64>,
) -> Vec<StorageValue> {
    let text =
        |value: Option<&str>| value.map_or(StorageValue::Null, |v| StorageValue::Text(v.into()));
    let number = |value: Option<i64>| value.map_or(StorageValue::Null, StorageValue::Int);
    vec![
        StorageValue::Int(1),
        number(attribute),
        StorageValue::Null,
        text(name1),
        text(name2),
        text(expression),
        number(flag),
        StorageValue::Null,
    ]
}

fn empty_query() -> NativeQuery {
    NativeQuery {
        name: "Q".to_owned(),
        kind: NativeQueryKind::Select,
        tables: Vec::new(),
        columns: Vec::new(),
        criteria: Vec::new(),
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }
}

#[test]
fn select_flags_and_parameters_decode() {
    let mut query = empty_query();
    apply_attribute(&row(Some(1), None, None, None, Some(0x10)), &mut query).expect("flags");
    assert_eq!(query.select_flags, 0x10);
    apply_attribute(
        &row(Some(2), Some("Which"), None, None, Some(10)),
        &mut query,
    )
    .expect("parameter");
    assert_eq!(query.parameters, ["Which"]);
}

#[test]
fn every_join_flavor_decodes_and_unknown_flags_are_refused() {
    for (flag, expected) in [
        (1, NativeJoinKind::Inner),
        (2, NativeJoinKind::Left),
        (3, NativeJoinKind::Right),
    ] {
        let mut query = empty_query();
        apply_attribute(
            &row(Some(7), Some("A"), Some("B"), Some("A.x = B.y"), Some(flag)),
            &mut query,
        )
        .expect("join");
        assert_eq!(query.joins[0].kind, expected);
        assert_eq!(query.joins[0].expression, "A.x = B.y");
    }

    let mut query = empty_query();
    let error = apply_attribute(
        &row(Some(7), None, None, Some("A.x = B.y"), Some(4)),
        &mut query,
    )
    .expect_err("unknown join flag");
    assert!(error.to_string().contains("join flag 4"), "{error}");

    let error = apply_attribute(&row(Some(7), None, None, None, Some(1)), &mut query)
        .expect_err("join without expression");
    assert!(
        error.to_string().contains("without an expression"),
        "{error}"
    );
}

#[test]
fn where_group_by_and_having_are_separated() {
    let mut query = empty_query();
    for (attribute, expression) in [(8, "a > 1"), (9, "b"), (10, "Count(*) > 1")] {
        apply_attribute(
            &row(Some(attribute), None, None, Some(expression), None),
            &mut query,
        )
        .expect("attribute");
    }
    assert_eq!(query.criteria, ["a > 1"]);
    assert_eq!(query.group_by, ["b"]);
    assert_eq!(query.having, ["Count(*) > 1"]);
}

#[test]
fn order_by_reads_the_expression_and_falls_back_to_name1() {
    let mut query = empty_query();
    apply_attribute(&row(Some(11), None, None, Some("T.c"), Some(1)), &mut query)
        .expect("order by");
    assert_eq!(query.order_by[0].field, "T.c");
    assert!(query.order_by[0].descending);

    apply_attribute(&row(Some(11), Some("c"), None, None, None), &mut query).expect("fallback");
    assert_eq!(query.order_by[1].field, "c");
    assert!(!query.order_by[1].descending);

    let error =
        apply_attribute(&row(Some(11), None, None, None, None), &mut query).expect_err("no field");
    assert!(
        error.to_string().contains("ORDER BY row without a field"),
        "{error}"
    );
}

#[test]
fn unknown_attributes_and_missing_attributes_are_precise() {
    let mut query = empty_query();
    let error = apply_attribute(&row(Some(42), None, None, Some("x"), None), &mut query)
        .expect_err("unknown attribute");
    assert!(error.to_string().contains("attribute code 42"), "{error}");

    let error = apply_attribute(&row(None, None, None, None, None), &mut query)
        .expect_err("missing attribute");
    assert!(matches!(error, StorageError::Corrupt { .. }), "{error:?}");
}

#[test]
fn helper_accessors_reject_non_integer_cells() {
    assert_eq!(object_id(&[StorageValue::Text("x".into())]), None);
    assert_eq!(
        attribute(&[StorageValue::Int(1), StorageValue::Text("x".into())]),
        None
    );
    assert_eq!(text(&[StorageValue::Null], 0), None);
    assert_eq!(text(&[StorageValue::Text(String::new())], 0), None);
    assert_eq!(flag(&[StorageValue::Null]), 0);
    assert_eq!(
        object_id(&[StorageValue::Int(7)]),
        Some(7),
        "integer ids decode"
    );
}

#[test]
fn unknown_query_flavors_are_unsupported() {
    let error = NativeQueryKind::from_flags(0x90).expect_err("unknown flavor");
    assert!(
        error.to_string().contains("query object flags 0x90"),
        "{error}"
    );
}
