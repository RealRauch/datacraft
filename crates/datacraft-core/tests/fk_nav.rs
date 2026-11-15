//! Foreign-key follow resolution (step 87.1, item M103).
//!
//! The pure core model behind the datasheet follow affordance: declared
//! forward keys (grouped composite keys stay paired), reverse keys
//! (referenced-by) and inferred `<table>_id` references — plus the
//! criteria encoder that turns the source row's key values into filter
//! criteria for the target table. The encoder round-trips through the
//! real criteria parser (step 7.3), so both sides of the wire share one
//! fixture set.

use datacraft_core::fk_nav::{FkDirection, follow_criteria, follow_targets};
use datacraft_core::port::{Relationship, RelationshipGroup};
use datacraft_core::query_criteria::{Predicate, parse_predicate};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_types::parse_date;

fn rel(child: &str, col: &str, parent: &str, pcol: &str) -> Relationship {
    Relationship {
        table: child.to_owned(),
        column: col.to_owned(),
        related_table: parent.to_owned(),
        related_column: pcol.to_owned(),
    }
}

/// One declared composite constraint: the child table's columns follow
/// the parent table's columns as one key.
fn group(
    name: Option<&str>,
    child: &str,
    parent: &str,
    pairs: [(&str, &str); 2],
) -> RelationshipGroup {
    RelationshipGroup {
        name: name.map(str::to_owned),
        table: child.to_owned(),
        related_table: parent.to_owned(),
        pairs: pairs
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect(),
    }
}

fn field(name: &str, kind: FieldKind) -> FieldDef {
    FieldDef::new(name, kind)
}

fn long() -> FieldKind {
    FieldKind::Number(NumericKind::LongInteger)
}

/// Orders(customer_id, bike_id), Customers(id, name), Bikes(id, model),
/// Keys(k1, k2), Other(o1, o2).
fn shop_tables() -> Vec<TableDef> {
    vec![
        TableDef::new(
            "Orders",
            vec![
                field("id", long()),
                field("customer_id", long()),
                field("bike_id", long()),
                field("note", FieldKind::Text { max_len: 50 }),
            ],
        ),
        TableDef::new(
            "Customers",
            vec![
                field("id", long()),
                field("name", FieldKind::Text { max_len: 50 }),
            ],
        ),
        TableDef::new(
            "Bikes",
            vec![
                field("id", long()),
                field("model", FieldKind::Text { max_len: 50 }),
            ],
        ),
        TableDef::new(
            "Keys",
            vec![field("k1", long()), field("k2", long())],
        ),
        TableDef::new(
            "Other",
            vec![field("o1", long()), field("o2", long())],
        ),
    ]
}

/// A declared single-column key resolves to the parent record's table.
#[test]
fn declared_forward_follows_to_the_parent_table() {
    let relationships = vec![rel("Orders", "customer_id", "Customers", "id")];
    let targets = follow_targets("Orders", "customer_id", &relationships, None, &shop_tables());
    assert_eq!(targets.len(), 1, "one declared target");
    assert_eq!(targets[0].direction, FkDirection::Forward);
    assert_eq!(targets[0].table, "Customers");
    assert_eq!(
        targets[0].pairs,
        vec![("customer_id".to_owned(), "id".to_owned())]
    );
}

/// Composite keys grouped by their constraint stay paired: the follow
/// carries every column pair of the group, never a single column of it
/// — and a second constraint's column between other tables resolves on
/// its own.
#[test]
fn grouped_composite_keys_stay_paired() {
    let groups = vec![
        group(Some("r1"), "Orders", "Keys", [("a", "k1"), ("b", "k2")]),
        group(Some("r2"), "Orders", "Other", [("c", "o1"), ("c2", "o2")]),
    ];
    let relationships = vec![
        rel("Orders", "a", "Keys", "k1"),
        rel("Orders", "b", "Keys", "k2"),
        rel("Orders", "c", "Other", "o1"),
        rel("Orders", "c2", "Other", "o2"),
    ];
    let targets = follow_targets("Orders", "a", &relationships, Some(&groups), &shop_tables());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].table, "Keys");
    assert_eq!(
        targets[0].pairs,
        vec![
            ("a".to_owned(), "k1".to_owned()),
            ("b".to_owned(), "k2".to_owned()),
        ],
        "the follow carries the whole composite group"
    );
    let targets = follow_targets("Orders", "c", &relationships, Some(&groups), &shop_tables());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].table, "Other");
    assert_eq!(
        targets[0].pairs,
        vec![
            ("c".to_owned(), "o1".to_owned()),
            ("c2".to_owned(), "o2".to_owned()),
        ]
    );
}

/// Two ungrouped entries between the same tables (a driver that carries
/// no constraint names) form one composite — the documented boundary of
/// pair grouping, pinned so the behaviour stays a decision.
#[test]
fn ungrouped_entries_between_the_same_tables_form_one_composite() {
    let relationships = vec![
        rel("Orders", "a", "Keys", "k1"),
        rel("Orders", "b", "Keys", "k2"),
    ];
    let targets = follow_targets("Orders", "a", &relationships, None, &shop_tables());
    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets[0].pairs,
        vec![
            ("a".to_owned(), "k1".to_owned()),
            ("b".to_owned(), "k2".to_owned()),
        ],
        "same-pair ungrouped entries follow together (documented boundary)"
    );
}

/// The parent side lists the referencing children as reverse targets.
#[test]
fn reverse_targets_list_the_referencing_children() {
    let relationships = vec![
        rel("Orders", "customer_id", "Customers", "id"),
        rel("OrdersParts", "order_id", "Orders", "id"),
    ];
    let targets = follow_targets("Customers", "id", &relationships, None, &shop_tables());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].direction, FkDirection::Reverse);
    assert_eq!(targets[0].table, "Orders");
    assert_eq!(
        targets[0].pairs,
        vec![("id".to_owned(), "customer_id".to_owned())],
        "reverse pairs map this table's column onto the child's column"
    );
}

/// A `_id` column without a declared key falls back to the inferred
/// reference: the `<base>` table exists and carries an `id` field.
#[test]
fn inferred_references_fall_back_without_declared_keys() {
    let targets = follow_targets("Orders", "bike_id", &[], None, &shop_tables());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].direction, FkDirection::Inferred);
    assert_eq!(targets[0].table, "Bikes");
    assert_eq!(
        targets[0].pairs,
        vec![("bike_id".to_owned(), "id".to_owned())]
    );
}

/// Inference never hallucinates: no `_id` suffix, no matching table or
/// no `id` field on the target — no inferred reference.
#[test]
fn inference_requires_suffix_base_table_and_id_field() {
    // No suffix: "note" is not a key column.
    assert!(follow_targets("Orders", "note", &[], None, &shop_tables()).is_empty());
    // Suffix but no base table: "parts_id" → "Parts" does not exist.
    assert!(follow_targets("Orders", "parts_id", &[], None, &shop_tables()).is_empty());
    // Suffix and table but no `id` field: "keys_id" → "Keys" has only
    // k1/k2.
    assert!(follow_targets("Orders", "keys_id", &[], None, &shop_tables()).is_empty());
    // A declared target suppresses inference for the same column.
    let relationships = vec![rel("Orders", "customer_id", "Customers", "id")];
    let targets = follow_targets("Orders", "customer_id", &relationships, None, &shop_tables());
    assert_eq!(targets.len(), 1, "declared wins, no duplicate inferred");
    assert_eq!(targets[0].direction, FkDirection::Forward);
}

/// Key values encode into criteria the real parser reads back as exact
/// equality — integers, floats, currency, decimals, booleans, dates and
/// text (wildcards stay literal under the explicit `=` form).
#[test]
fn follow_criteria_encodes_for_the_real_parser() {
    let tables = shop_tables();
    let target = follow_targets("Orders", "customer_id", &[], None, &tables)
        .into_iter()
        .next()
        .expect("inferred target");
    let cases = [
        FieldValue::Integer(7),
        FieldValue::Float(2.5),
        FieldValue::Currency(125_000),
        FieldValue::Decimal(1250),
        FieldValue::Boolean(true),
        parse_date("2026-01-02").expect("date"),
        FieldValue::Text("A*B".to_owned()),
        FieldValue::Text("plain".to_owned()),
    ];
    for value in cases {
        let values = vec![("customer_id".to_owned(), Some(value.clone()))];
        let criteria =
            follow_criteria(&target, &values, &tables).expect("every supported value encodes");
        assert_eq!(criteria.len(), 1);
        let (column, text) = &criteria[0];
        assert_eq!(column, "id");
        let def = FieldDef::new("id", long());
        let parsed = parse_predicate(text, "id", &def).expect("the parser reads the encoding");
        assert_eq!(parsed, Predicate::Equal(value.clone()), "criteria {text:?} re-parses exactly");
        assert!(
            parsed.matches(&value),
            "the parsed predicate matches the source value itself"
        );
    }
}

/// A NULL key — fully or partially (composite) — answers a precise
/// message instead of following to a wrong pairing.
#[test]
fn follow_criteria_refuses_null_and_partial_composite_keys() {
    let tables = shop_tables();
    let relationships = vec![
        rel("Orders", "customer_id", "Customers", "id"),
        rel("Orders", "bike_id", "Bikes", "id"),
    ];
    let target = follow_targets("Orders", "customer_id", &relationships, None, &tables)
        .into_iter()
        .next()
        .expect("declared target");
    let all_null = vec![("customer_id".to_owned(), None)];
    let reason = follow_criteria(&target, &all_null, &tables).expect_err("NULL refuses");
    assert!(
        reason.to_string().contains("customer_id"),
        "the message names the empty column: {reason}"
    );
    let composite_target = datacraft_core::fk_nav::FkFollowTarget {
        direction: FkDirection::Forward,
        table: "Keys".to_owned(),
        pairs: vec![
            ("a".to_owned(), "k1".to_owned()),
            ("b".to_owned(), "k2".to_owned()),
        ],
    };
    let partial = vec![
        ("a".to_owned(), Some(FieldValue::Integer(5))),
        ("b".to_owned(), None),
    ];
    let keys_tables = vec![tables[3].clone()];
    let reason = follow_criteria(&composite_target, &partial, &keys_tables)
        .expect_err("a partially empty composite refuses");
    assert!(
        reason.to_string().contains("b"),
        "the message names the partially empty column: {reason}"
    );
}

/// Values the criteria dialect cannot express exactly (GUID, binary,
/// complex, quote-wrapped text) refuse with a precise message instead of
/// degrading to a wrong filter.
#[test]
fn follow_criteria_refuses_unencodable_values() {
    let tables = shop_tables();
    let target = follow_targets("Orders", "customer_id", &[], None, &tables)
        .into_iter()
        .next()
        .expect("inferred target");
    let unencodable = [
        FieldValue::Guid([0u8; 16]),
        FieldValue::Binary(vec![1, 2, 3]),
        FieldValue::Text("\"wrapped\"".to_owned()),
    ];
    for value in unencodable {
        let values = vec![("customer_id".to_owned(), Some(value))];
        let reason =
            follow_criteria(&target, &values, &tables).expect_err("unencodable values refuse");
        assert!(
            reason.to_string().contains("customer_id"),
            "the message names the column: {reason}"
        );
    }
}
