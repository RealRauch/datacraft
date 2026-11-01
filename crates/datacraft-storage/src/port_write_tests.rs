//! Unit tests for the domain -> Jet write mapping (step 11.1).
//!
//! The lossiness table lives in `port_write.rs`; these tests pin every arm
//! of the mapping so a kind cannot silently change its storage shape.

use super::*;
use crate::jet::tdef::flags;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn column(field: &FieldDef) -> JetColumn {
    table_to_columns(&TableDef::new("T", vec![field.clone()]))
        .expect("maps")
        .pop()
        .expect("one column")
}

#[test]
fn every_supported_kind_maps_to_its_storage_shape() {
    let cases: &[(FieldKind, JetType, u16, bool, u8)] = &[
        (FieldKind::Text { max_len: 10 }, JetType::Text, 20, false, 0),
        (FieldKind::Memo, JetType::Memo, 0, false, 0),
        (
            FieldKind::Number(NumericKind::Byte),
            JetType::Byte,
            1,
            true,
            0,
        ),
        (
            FieldKind::Number(NumericKind::Integer),
            JetType::Int,
            2,
            true,
            0,
        ),
        (
            FieldKind::Number(NumericKind::LongInteger),
            JetType::LongInt,
            4,
            true,
            0,
        ),
        (
            FieldKind::Number(NumericKind::Single),
            JetType::Float,
            4,
            true,
            0,
        ),
        (
            FieldKind::Number(NumericKind::Double),
            JetType::Double,
            8,
            true,
            0,
        ),
        (
            FieldKind::Number(NumericKind::Decimal {
                precision: 6,
                scale: 2,
            }),
            JetType::Numeric,
            17,
            true,
            2,
        ),
        (FieldKind::DateTime, JetType::DateTime, 8, true, 0),
        (FieldKind::Currency, JetType::Currency, 8, true, 0),
        (
            FieldKind::Boolean { tri_state: false },
            JetType::Bool,
            0,
            true,
            0,
        ),
        (
            FieldKind::Binary(BinaryKind::OleObject),
            JetType::Ole,
            0,
            false,
            0,
        ),
        (FieldKind::Guid, JetType::Guid, 16, true, 0),
    ];
    for (kind, data_type, len, fixed, scale) in cases {
        let col = column(&FieldDef::new("f", *kind));
        assert_eq!(col.data_type, *data_type, "{kind:?}");
        assert_eq!(col.len, *len, "{kind:?}");
        assert_eq!(col.is_fixed(), *fixed, "{kind:?}");
        assert_eq!(col.scale, *scale, "{kind:?}");
        assert!(col.is_nullable(), "Access defaults to nullable: {kind:?}");
    }
}

#[test]
fn required_fields_are_not_nullable_and_ids_follow_the_order() {
    let mut required = FieldDef::new("a", FieldKind::Memo);
    required.properties.required = true;
    let columns = table_to_columns(&TableDef::new(
        "T",
        vec![required, FieldDef::new("b", FieldKind::Memo)],
    ))
    .expect("maps");
    assert_eq!(columns[0].flags & flags::NULLABLE, 0);
    assert_ne!(columns[1].flags & flags::NULLABLE, 0);
    assert_eq!(columns[0].column_id, 0);
    assert_eq!(columns[1].column_id, 1);
}

#[test]
fn unsupported_kinds_are_precise() {
    for kind in [
        FieldKind::AutoNumber(AutoKind::LongInteger),
        FieldKind::AutoNumber(AutoKind::ReplicationId),
        FieldKind::Binary(BinaryKind::Attachment),
    ] {
        let err = table_to_columns(&TableDef::new("T", vec![FieldDef::new("f", kind)]))
            .expect_err("kind is unsupported for the writer");
        assert!(
            matches!(err, PortError::Unsupported { .. }),
            "{kind:?}: {err:?}"
        );
    }

    let invalid = TableDef::new("", vec![FieldDef::new("f", FieldKind::Memo)]);
    let err = table_to_columns(&invalid).expect_err("invalid definitions fail");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
}

#[test]
fn values_map_one_to_one_and_arity_is_checked() {
    let fields = vec![
        FieldDef::new("text", FieldKind::Text { max_len: 8 }),
        FieldDef::new("memo", FieldKind::Memo),
        FieldDef::new("flag", FieldKind::Boolean { tri_state: false }),
        FieldDef::new("blob", FieldKind::Binary(BinaryKind::OleObject)),
        FieldDef::new("guid", FieldKind::Guid),
    ];
    let values = vec![
        FieldValue::Text("x".into()),
        FieldValue::Null,
        FieldValue::Boolean(true),
        FieldValue::Binary(vec![1]),
        FieldValue::Guid([2; 16]),
    ];
    assert_eq!(
        to_storage_values(&fields, &values).expect("maps"),
        vec![
            StorageValue::Text("x".into()),
            StorageValue::Null,
            StorageValue::Bit(true),
            StorageValue::Binary(vec![1]),
            StorageValue::Guid([2; 16]),
        ]
    );

    let err = to_storage_values(&fields, &values[..1]).expect_err("arity mismatch");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");

    let wrong_family = vec![
        FieldValue::Integer(1),
        FieldValue::Null,
        FieldValue::Boolean(true),
        FieldValue::Binary(vec![]),
        FieldValue::Guid([0; 16]),
    ];
    let err = to_storage_values(&fields, &wrong_family).expect_err("family mismatch");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
}

#[test]
fn autonumber_values_are_rejected_before_storage() {
    let fields = vec![FieldDef::new(
        "id",
        FieldKind::AutoNumber(AutoKind::LongInteger),
    )];
    let err =
        to_storage_values(&fields, &[FieldValue::Null]).expect_err("AutoNumber is engine-assigned");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}
