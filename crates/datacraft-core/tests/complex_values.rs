//! Domain model and query semantics of ACE complex values (step 16.1, M4).
//!
//! Complex cells are ordered sub-records: attachments (FileName/FileData/…),
//! multi-value columns (Value) and generic complex records (version history).
//! QBE predicates over a multi-value column use EXISTS semantics: a row
//! matches when at least one element satisfies the predicate.

use datacraft_core::present::{display_value, field_kind_display};
use datacraft_core::qbe::{QbeField, QueryDef, SortOrder};
use datacraft_core::query_engine::run_query;
use datacraft_core::schema::{
    BinaryKind, ComplexField, ComplexRecord, FieldDef, FieldKind, FieldValue, MultiValueKind,
    NumericKind, TableDef,
};

fn record(fields: &[(&str, FieldValue)]) -> ComplexRecord {
    ComplexRecord {
        fields: fields
            .iter()
            .map(|(name, value)| ComplexField {
                name: (*name).to_owned(),
                value: value.clone(),
            })
            .collect(),
    }
}

fn text_record(name: &str, value: &str) -> ComplexRecord {
    record(&[(name, FieldValue::Text(value.to_owned()))])
}

fn multi_value_table() -> TableDef {
    TableDef::new(
        "T",
        vec![FieldDef::new(
            "tags",
            FieldKind::MultiValue(MultiValueKind::Text { max_len: 255 }),
        )],
    )
}

fn rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![FieldValue::Complex(Vec::new())],
        vec![FieldValue::Complex(vec![text_record("Value", "a")])],
        vec![FieldValue::Complex(vec![
            text_record("Value", "a"),
            text_record("Value", "b"),
        ])],
        vec![FieldValue::Complex(vec![text_record("Value", "b")])],
    ]
}

fn query(criteria: &str) -> QueryDef {
    QueryDef {
        name: "q".to_owned(),
        table: "T".to_owned(),
        fields: vec![QbeField {
            name: "tags".to_owned(),
            shown: true,
            sort: SortOrder::None,
            criteria: vec![criteria.to_owned()],
        }],
    }
}

#[test]
fn multi_value_criteria_use_exists_semantics() {
    let table = multi_value_table();
    let rows = rows();

    // Equality: any element equals the literal.
    let equal = run_query(&query("a"), &table, &rows).expect("query runs");
    assert_eq!(equal.rows.len(), 2, "{:?}", equal.rows);

    // Not-equal: any element differs from the literal (EXISTS semantics —
    // [a] has no differing element and does not match).
    let not_equal = run_query(&query("<>a"), &table, &rows).expect("query runs");
    assert_eq!(not_equal.rows.len(), 2, "{:?}", not_equal.rows);

    // No element matches, and the empty list never matches.
    let missing = run_query(&query("z"), &table, &rows).expect("query runs");
    assert!(missing.rows.is_empty(), "{:?}", missing.rows);
}

#[test]
fn complex_values_render_as_joined_scalars() {
    let attachment = FieldValue::Complex(vec![
        record(&[
            ("FileName", FieldValue::Text("a.txt".into())),
            ("FileData", FieldValue::Binary(vec![1, 2])),
        ]),
        record(&[
            ("FileName", FieldValue::Text("b.txt".into())),
            ("FileData", FieldValue::Binary(vec![3])),
        ]),
    ]);
    assert_eq!(display_value(&attachment), "a.txt, b.txt");

    let multivalue =
        FieldValue::Complex(vec![text_record("Value", "x"), text_record("Value", "y")]);
    assert_eq!(display_value(&multivalue), "x, y");

    assert_eq!(display_value(&FieldValue::Complex(Vec::new())), "");
}

#[test]
fn the_scalar_of_a_record_prefers_value_then_file_name() {
    let multivalue = record(&[
        ("Table1_multi-value-data", FieldValue::Integer(1)),
        ("Value", FieldValue::Text("x".into())),
    ]);
    assert_eq!(multivalue.scalar(), Some(&FieldValue::Text("x".into())));

    let attachment = record(&[
        ("FileData", FieldValue::Binary(vec![9])),
        ("FileName", FieldValue::Text("a.txt".into())),
    ]);
    assert_eq!(attachment.scalar(), Some(&FieldValue::Text("a.txt".into())));

    // Generic complex record: first non-binary field.
    let history = record(&[
        ("append-memo-data", FieldValue::Text("memo".into())),
        ("Modified", FieldValue::DateTime(1.0)),
    ]);
    assert_eq!(history.scalar(), Some(&FieldValue::Text("memo".into())));
}

#[test]
fn complex_kinds_display_and_validate() {
    assert_eq!(
        field_kind_display(&FieldKind::MultiValue(MultiValueKind::Text {
            max_len: 255
        })),
        "Multi-Value"
    );
    assert_eq!(field_kind_display(&FieldKind::Complex), "Complex");

    let multivalue = FieldKind::MultiValue(MultiValueKind::Text { max_len: 5 });
    assert!(
        multivalue
            .validate_value(&FieldValue::Complex(vec![text_record("Value", "abc")]))
            .is_ok()
    );
    assert!(
        multivalue
            .validate_value(&FieldValue::Complex(vec![text_record("Value", "toolong")]))
            .is_err()
    );
    assert!(
        multivalue
            .validate_value(&FieldValue::Binary(vec![]))
            .is_err()
    );
    assert!(multivalue.validate_value(&FieldValue::Null).is_ok());

    let numeric = FieldKind::MultiValue(MultiValueKind::Number(NumericKind::LongInteger));
    assert!(
        numeric
            .validate_value(&FieldValue::Complex(vec![record(&[(
                "Value",
                FieldValue::Integer(7)
            )])]))
            .is_ok()
    );

    assert!(
        FieldKind::Complex
            .validate_value(&FieldValue::Complex(vec![text_record("Value", "x")]))
            .is_ok()
    );
    assert!(
        FieldKind::Binary(BinaryKind::Attachment)
            .validate_value(&FieldValue::Complex(vec![text_record("FileName", "a.txt")]))
            .is_ok()
    );
    assert!(
        FieldKind::Binary(BinaryKind::OleObject)
            .validate_value(&FieldValue::Binary(vec![1]))
            .is_ok()
    );
    assert!(
        FieldKind::Binary(BinaryKind::OleObject)
            .validate_value(&FieldValue::Complex(vec![]))
            .is_err()
    );
}

#[test]
fn complex_values_report_their_family() {
    assert_eq!(FieldValue::Complex(vec![]).family(), "complex");
}
