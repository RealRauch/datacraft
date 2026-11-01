use super::*;
use crate::schema::{ComplexField, ComplexRecord};

/// Table-driven: every kind rejects a value of the wrong family with a
/// mismatch naming both sides (covers all mismatch arms and family
/// names end-to-end).
#[test]
fn every_kind_rejects_a_wrong_family_value() {
    let cases: &[(FieldKind, FieldValue)] = &[
        (FieldKind::Text { max_len: 5 }, FieldValue::Integer(1)),
        (FieldKind::Memo, FieldValue::Binary(vec![])),
        (
            FieldKind::Number(NumericKind::Byte),
            FieldValue::Text("1".into()),
        ),
        (
            FieldKind::Number(NumericKind::Integer),
            FieldValue::Text("1".into()),
        ),
        (
            FieldKind::Number(NumericKind::LongInteger),
            FieldValue::Boolean(true),
        ),
        (
            FieldKind::Number(NumericKind::Single),
            FieldValue::Integer(1),
        ),
        (
            FieldKind::Number(NumericKind::Double),
            FieldValue::Integer(1),
        ),
        (
            FieldKind::Number(NumericKind::Decimal {
                precision: 5,
                scale: 2,
            }),
            FieldValue::Integer(1),
        ),
        (FieldKind::DateTime, FieldValue::Text("2020".into())),
        (FieldKind::Currency, FieldValue::Integer(1)),
        (
            FieldKind::Boolean { tri_state: false },
            FieldValue::Integer(1),
        ),
        (
            FieldKind::Binary(BinaryKind::OleObject),
            FieldValue::Text("x".into()),
        ),
        (FieldKind::Guid, FieldValue::Integer(1)),
    ];
    for (kind, value) in cases {
        let err = kind
            .validate_value(value)
            .expect_err("wrong family must be rejected");
        match err {
            SchemaError::KindMismatch { expected, found } => {
                assert_eq!(expected, kind.family(), "expected side for {kind:?}");
                assert_eq!(found, value.family(), "found side for {value:?}");
            }
            other => panic!("expected KindMismatch for {kind:?}, got {other:?}"),
        }
    }
}

/// Multi-value kinds validate every element against their scalar family
/// and cover the element-kind mapping (step 16.1, M4).
#[test]
fn multi_value_kinds_validate_their_element_families() {
    let text = FieldKind::MultiValue(MultiValueKind::Text { max_len: 5 });
    assert!(text.validate().is_ok());
    assert!(
        FieldKind::MultiValue(MultiValueKind::Text { max_len: 0 })
            .validate()
            .is_err()
    );
    assert!(
        FieldKind::MultiValue(MultiValueKind::Number(NumericKind::Decimal {
            precision: 0,
            scale: 0
        }))
        .validate()
        .is_err()
    );
    assert!(
        text.validate_value(&FieldValue::Complex(vec![ComplexRecord { fields: vec![] }]))
            .is_err(),
        "a record without a scalar is not a multi-value element"
    );
    assert!(
        text.validate_value(&FieldValue::Complex(vec![ComplexRecord {
            fields: vec![ComplexField {
                name: "Value".to_owned(),
                value: FieldValue::Text("toolong".to_owned()),
            }],
        }]))
        .is_err()
    );
    assert!(text.validate_value(&FieldValue::Integer(1)).is_err());
    let cases = [
        (MultiValueKind::Text { max_len: 1 }, "short text"),
        (MultiValueKind::Number(NumericKind::Byte), "integer"),
        (MultiValueKind::DateTime, "date/time"),
        (MultiValueKind::Currency, "currency"),
        (MultiValueKind::Boolean { tri_state: true }, "boolean"),
        (MultiValueKind::Guid, "guid"),
    ];
    for (kind, family) in cases {
        assert_eq!(kind.element_kind().family(), family);
    }
}

/// Attachment fields accept complex values (step 16.1, M4) and report
/// their binary family; plain binary values stay OLE-object-only.
#[test]
fn attachment_kind_accepts_complex_values() {
    let kind = FieldKind::Binary(BinaryKind::Attachment);
    assert!(
        kind.validate_value(&FieldValue::Complex(vec![ComplexRecord {
            fields: vec![ComplexField {
                name: "FileName".to_owned(),
                value: FieldValue::Text("a.txt".to_owned()),
            }],
        }]))
        .is_ok()
    );
    assert!(kind.validate_value(&FieldValue::Binary(vec![9])).is_err());
    assert_eq!(kind.family(), "binary");
}
