//! Filter builder model (step 87.3, item M104).
//!
//! The structured filter: conditions and AND/OR groups with visible
//! parentheses, ordered sort columns, and the serialization onto the
//! existing criteria map — the execution path and the wildcard rule
//! stay unchanged, so the encoder round-trips through the real
//! criteria parser (step 7.3). OR groups are outside the dialect: the
//! refusal is precise, an invalid draft never runs.

use std::collections::BTreeMap;

use datacraft_core::filter_builder::{
    BuilderCondition, BuilderOp, BuilderSort, Connective, FilterBuilderError, FilterNode,
    deserialize_filters, serialize_filters, serialize_sorts,
};
use datacraft_core::qbe::SortOrder;
use datacraft_core::query_criteria::{Predicate, parse_predicate};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind};

fn cond(column: &str, op: BuilderOp, literal: &str) -> FilterNode {
    FilterNode::Condition(BuilderCondition {
        column: column.to_owned(),
        op,
        literal: literal.to_owned(),
    })
}

fn and(children: Vec<FilterNode>) -> FilterNode {
    FilterNode::Group {
        connective: Connective::And,
        children,
    }
}

fn or(children: Vec<FilterNode>) -> FilterNode {
    FilterNode::Group {
        connective: Connective::Or,
        children,
    }
}

fn def(name: &str, kind: FieldKind) -> FieldDef {
    FieldDef::new(name, kind)
}

fn long() -> FieldKind {
    FieldKind::Number(NumericKind::LongInteger)
}

fn text() -> FieldKind {
    FieldKind::Text { max_len: 50 }
}

/// An AND group serializes one criteria per column — the existing map
/// the datasheet's view order applies.
#[test]
fn and_conditions_serialize_per_column() {
    let root = and(vec![
        cond("amount", BuilderOp::Greater, "100"),
        cond("name", BuilderOp::Equal, "Ada"),
    ]);
    let filters = serialize_filters(&root).expect("AND groups serialize");
    assert_eq!(
        filters,
        BTreeMap::from([
            ("amount".to_owned(), "> 100".to_owned()),
            ("name".to_owned(), "= Ada".to_owned()),
        ])
    );
}

/// Every supported operator's criteria re-parses as the same predicate
/// through the real parser — the execution path is untouched.
#[test]
fn every_operator_round_trips_through_the_real_parser() {
    let cases = [
        (BuilderOp::Equal, "42"),
        (BuilderOp::NotEqual, "42"),
        (BuilderOp::Less, "42"),
        (BuilderOp::LessEqual, "42"),
        (BuilderOp::Greater, "42"),
        (BuilderOp::GreaterEqual, "42"),
    ];
    for (op, literal) in cases {
        let root = cond("amount", op, literal);
        let filters = serialize_filters(&root).expect("the condition serializes");
        let criteria = &filters["amount"];
        let def = def("amount", long());
        let parsed =
            parse_predicate(criteria, "amount", &def).expect("the parser reads the encoding");
        match (parsed, op, literal) {
            (
                Predicate::Equal(datacraft_core::schema::FieldValue::Integer(42)),
                BuilderOp::Equal,
                _,
            ) => {}
            (Predicate::NotEqual(_), BuilderOp::NotEqual, _)
            | (Predicate::Less(_), BuilderOp::Less, _)
            | (Predicate::LessEqual(_), BuilderOp::LessEqual, _)
            | (Predicate::Greater(_), BuilderOp::Greater, _)
            | (Predicate::GreaterEqual(_), BuilderOp::GreaterEqual, _) => {}
            (parsed, _, _) => {
                panic!("{op:?} serialized to {criteria:?} but parsed as {parsed:?}")
            }
        }
    }
    // Like keeps the quoted pattern form and the wildcard rule.
    let root = cond("name", BuilderOp::Like, "A*");
    let filters = serialize_filters(&root).expect("Like serializes");
    assert_eq!(filters["name"], "Like \"A*\"", "the wildcard form stays");
    let def = def("name", text());
    let parsed = parse_predicate(&filters["name"], "name", &def).expect("the parser reads Like");
    assert_eq!(parsed, Predicate::Like("A*".to_owned()));
}

/// Nested AND groups flatten into the same per-column map (the
/// columns AND together — that is the execution path's own semantics).
#[test]
fn nested_and_groups_flatten_into_the_map() {
    let root = and(vec![
        cond("amount", BuilderOp::Greater, "100"),
        and(vec![
            cond("name", BuilderOp::Equal, "Ada"),
            cond("city", BuilderOp::NotEqual, "Rome"),
        ]),
    ]);
    let filters = serialize_filters(&root).expect("nested AND flattens");
    assert_eq!(filters.len(), 3);
    assert_eq!(filters["city"], "<> Rome");
}

/// An OR group is outside the criteria dialect: the serialization
/// refuses precisely — an invalid draft never runs.
#[test]
fn or_groups_refuse_with_a_precise_message() {
    let same_columns = or(vec![
        cond("amount", BuilderOp::Equal, "1"),
        cond("amount", BuilderOp::Equal, "2"),
    ]);
    let reason = serialize_filters(&same_columns).expect_err("OR refuses");
    assert!(
        reason.to_string().contains("OR"),
        "the message names the fence: {reason}"
    );
    let across_columns = or(vec![
        cond("amount", BuilderOp::Equal, "1"),
        cond("name", BuilderOp::Equal, "Ada"),
    ]);
    let reason = serialize_filters(&across_columns).expect_err("a cross-column OR refuses");
    assert!(
        reason.to_string().contains("OR"),
        "the message names the fence: {reason}"
    );
    // Nested inside an AND: still outside the dialect.
    let nested = and(vec![
        cond("name", BuilderOp::Equal, "Ada"),
        or(vec![
            cond("amount", BuilderOp::Equal, "1"),
            cond("amount", BuilderOp::Equal, "2"),
        ]),
    ]);
    assert!(serialize_filters(&nested).is_err(), "a nested OR refuses");
}

/// Two conditions on the same column inside one AND group cannot map
/// onto the one-criteria-per-column input — the refusal names it.
#[test]
fn duplicate_column_conditions_in_one_and_refuse() {
    let root = and(vec![
        cond("amount", BuilderOp::Greater, "1"),
        cond("amount", BuilderOp::Less, "9"),
    ]);
    let reason = serialize_filters(&root).expect_err("duplicate columns refuse");
    assert!(
        reason.to_string().contains("amount"),
        "the message names the column: {reason}"
    );
}

/// The model renders grouped precedence visibly parenthesized — the
/// builder UI's text form and the behaviour doc pin the same shape.
#[test]
fn groups_parenthesize_visibly() {
    let root = and(vec![
        cond("amount", BuilderOp::Greater, "100"),
        or(vec![
            cond("name", BuilderOp::Equal, "Ada"),
            cond("name", BuilderOp::Equal, "Grace"),
        ]),
    ]);
    let text = root.to_string();
    assert_eq!(
        text, "(amount > 100 AND (name = \"Ada\" OR name = \"Grace\"))",
        "the parenthesized form shows the grouping"
    );
}

/// The criteria map reads back into the builder: an AND group of
/// per-column conditions, operator forms and Like preserved; a bare
/// literal normalizes to the explicit `=` form.
#[test]
fn deserialize_reads_the_criteria_map_back() {
    let filters = BTreeMap::from([
        ("amount".to_owned(), "> 100".to_owned()),
        ("name".to_owned(), "Like \"A*\"".to_owned()),
        ("city".to_owned(), "Rome".to_owned()),
    ]);
    let node = match deserialize_filters(&filters) {
        datacraft_core::filter_builder::FilterRead::Model(node) => node,
        datacraft_core::filter_builder::FilterRead::Failed(reason) => {
            panic!("the modeled criteria reads back: {reason}")
        }
    };
    assert_eq!(
        node.to_string(),
        "(amount > 100 AND city = \"Rome\" AND name Like \"A*\")",
        "the map reads back as one AND group"
    );
    // The semantic round trip: serialize again, parse-equal for every
    // column.
    let again = serialize_filters(&node).expect("the read-back serializes");
    assert_eq!(again["amount"], "> 100");
    assert_eq!(again["name"], "Like \"A*\"");
    assert_eq!(again["city"], "= Rome", "the bare form normalizes");
}

/// A criteria form the builder cannot model (the dialect's recognized
/// Access forms) refuses with a precise pointer to the text mode.
#[test]
fn unmodelable_criteria_refuse_deserialization() {
    let filters = BTreeMap::from([("amount".to_owned(), "Between 1 and 9".to_owned())]);
    let reason = match deserialize_filters(&filters) {
        datacraft_core::filter_builder::FilterRead::Failed(reason) => reason.to_string(),
        _ => panic!("Between refuses deserialization"),
    };
    assert!(
        reason.contains("amount") && reason.to_lowercase().contains("text mode"),
        "the message names the column and the escape hatch: {reason}"
    );
}

/// The bare wildcard pattern is the dialect's implicit-Like form (step
/// 70.1, H58) — reading it as a condition would round-trip into an
/// equality, so it fences to the text mode as well.
#[test]
fn bare_wildcard_criteria_read_as_text_mode() {
    let filters = BTreeMap::from([("name".to_owned(), "A*".to_owned())]);
    let reason = match deserialize_filters(&filters) {
        datacraft_core::filter_builder::FilterRead::Failed(reason) => reason.to_string(),
        _ => panic!("the bare wildcard fences to the text mode"),
    };
    assert!(
        reason.contains("name") && reason.to_lowercase().contains("text mode"),
        "the message names the column and the escape hatch: {reason}"
    );
    // An explicit operator keeps the condition (the semantics stay).
    let filters = BTreeMap::from([("name".to_owned(), "= A*".to_owned())]);
    match deserialize_filters(&filters) {
        datacraft_core::filter_builder::FilterRead::Model(node) => {
            assert!(node.to_string().contains("name = \"A*\""));
        }
        other => panic!("an explicit condition models: {other:?}"),
    }
}

/// Sort columns stay ordered in the model; the existing single-column
/// path applies the first entry (the documented fence).
#[test]
fn sort_columns_stay_ordered() {
    let sorts = vec![
        BuilderSort {
            column: "name".to_owned(),
            order: SortOrder::Ascending,
        },
        BuilderSort {
            column: "amount".to_owned(),
            order: SortOrder::Descending,
        },
    ];
    assert_eq!(sorts[0].column, "name", "the model keeps the order");
    let applied = serialize_sorts(&sorts);
    assert_eq!(
        applied,
        Some(("name".to_owned(), SortOrder::Ascending)),
        "the existing single-column path applies the first sort"
    );
    assert_eq!(serialize_sorts(&[]), None, "no sorts apply nothing");
}

/// An empty AND group serializes to no filters (nothing to apply) and
/// a bare condition to its single column.
#[test]
fn empty_and_serializes_to_no_filters() {
    let filters = serialize_filters(&and(vec![])).expect("an empty AND serializes");
    assert!(filters.is_empty(), "nothing to apply");
    let filters = serialize_filters(&cond("amount", BuilderOp::Equal, "1"))
        .expect("a bare condition serializes");
    assert_eq!(filters["amount"], "= 1");
}

/// The error type carries the fence reasons as data (agent surfaces).
#[test]
fn fence_errors_are_typed() {
    let or_group = or(vec![
        cond("a", BuilderOp::Equal, "1"),
        cond("b", BuilderOp::Equal, "2"),
    ]);
    match serialize_filters(&or_group) {
        Err(FilterBuilderError::OrNotExpressible) => {}
        other => panic!("OR fences as OrNotExpressible, got {other:?}"),
    }
    let duplicates = and(vec![
        cond("a", BuilderOp::Equal, "1"),
        cond("a", BuilderOp::Equal, "2"),
    ]);
    match serialize_filters(&duplicates) {
        Err(FilterBuilderError::DuplicateColumn { column }) => assert_eq!(column, "a"),
        other => panic!("duplicates fence as DuplicateColumn, got {other:?}"),
    }
}
