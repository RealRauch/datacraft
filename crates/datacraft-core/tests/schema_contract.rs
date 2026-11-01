//! Contract suite for the Access domain schema model (step 3.1, item H10).
//!
//! Locks the semantics of `FieldKind`, `FieldProperties`, `FieldValue`,
//! `FieldDef` and `TableDef` before the implementation exists: Access type
//! families with their invariants, per-kind property applicability, value
//! validation and JSON round-trips (the sidecar document of step 3.2 builds
//! on exactly this vocabulary).

use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldProperties, FieldValue, IndexMode, NumericKind,
    SchemaError,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn err_contains(result: Result<(), SchemaError>, needle: &str) {
    let msg = result.expect_err("expected a schema error").to_string();
    assert!(
        msg.to_lowercase().contains(&needle.to_lowercase()),
        "error message {msg:?} does not mention {needle:?}"
    );
}

fn ok(result: Result<(), SchemaError>) {
    result.unwrap_or_else(|e| panic!("expected Ok, got: {e}"));
}

fn text_kind(max_len: u16) -> FieldKind {
    FieldKind::Text { max_len }
}

// ---------------------------------------------------------------------------
// kind invariants
// ---------------------------------------------------------------------------

#[test]
fn text_kind_accepts_lengths_1_to_255_only() {
    ok(text_kind(1).validate());
    ok(text_kind(255).validate());
    err_contains(text_kind(0).validate(), "max_len");
    err_contains(text_kind(256).validate(), "max_len");
}

#[test]
fn decimal_kind_requires_precision_1_to_28_and_scale_le_precision() {
    ok(FieldKind::Number(NumericKind::Decimal {
        precision: 1,
        scale: 0,
    })
    .validate());
    ok(FieldKind::Number(NumericKind::Decimal {
        precision: 28,
        scale: 28,
    })
    .validate());
    ok(FieldKind::Number(NumericKind::Decimal {
        precision: 18,
        scale: 4,
    })
    .validate());
    err_contains(
        FieldKind::Number(NumericKind::Decimal {
            precision: 0,
            scale: 0,
        })
        .validate(),
        "precision",
    );
    err_contains(
        FieldKind::Number(NumericKind::Decimal {
            precision: 29,
            scale: 0,
        })
        .validate(),
        "precision",
    );
    err_contains(
        FieldKind::Number(NumericKind::Decimal {
            precision: 10,
            scale: 11,
        })
        .validate(),
        "scale",
    );
}

#[test]
fn default_properties_validate_for_every_kind() {
    let kinds = [
        text_kind(50),
        FieldKind::Memo,
        FieldKind::Number(NumericKind::Byte),
        FieldKind::Number(NumericKind::Integer),
        FieldKind::Number(NumericKind::LongInteger),
        FieldKind::Number(NumericKind::Single),
        FieldKind::Number(NumericKind::Double),
        FieldKind::Number(NumericKind::Decimal {
            precision: 18,
            scale: 2,
        }),
        FieldKind::DateTime,
        FieldKind::Currency,
        FieldKind::AutoNumber(AutoKind::LongInteger),
        FieldKind::AutoNumber(AutoKind::ReplicationId),
        FieldKind::Boolean { tri_state: false },
        FieldKind::Boolean { tri_state: true },
        FieldKind::Binary(BinaryKind::OleObject),
        FieldKind::Binary(BinaryKind::Attachment),
    ];
    for kind in kinds {
        ok(FieldProperties::default().validate_for(&kind));
    }
}

#[test]
fn allow_zero_length_is_text_only() {
    let mut props = FieldProperties::default();
    props.allow_zero_length = true;
    ok(props.validate_for(&text_kind(10)));
    ok(props.validate_for(&FieldKind::Memo));
    err_contains(props.validate_for(&FieldKind::DateTime), "allow_zero_length");
    err_contains(
        props.validate_for(&FieldKind::Number(NumericKind::LongInteger)),
        "allow_zero_length",
    );
}

#[test]
fn expression_properties_must_not_be_empty_strings() {
    for setter in [
        |p: &mut FieldProperties| p.input_mask = Some(String::new()),
        |p: &mut FieldProperties| p.format = Some(String::new()),
        |p: &mut FieldProperties| p.default_value = Some(String::new()),
        |p: &mut FieldProperties| p.validation_rule = Some(String::new()),
        |p: &mut FieldProperties| p.validation_text = Some(String::new()),
    ] {
        let mut props = FieldProperties::default();
        setter(&mut props);
        err_contains(
            props.validate_for(&text_kind(10)),
            "empty",
        );
    }
    // The caption is display text, not an expression — empty is allowed.
    let mut props = FieldProperties::default();
    props.caption = Some(String::new());
    ok(props.validate_for(&text_kind(10)));
}

// ---------------------------------------------------------------------------
// value validation per kind
// ---------------------------------------------------------------------------

#[test]
fn null_passes_kind_validation_regardless_of_required() {
    // `required` is a property concern checked on FieldDef; the kind itself
    // accepts Null and leaves the decision to the field definition.
    ok(text_kind(10).validate_value(&FieldValue::Null));
    ok(
        FieldKind::Boolean { tri_state: false }.validate_value(&FieldValue::Null),
    );
}

#[test]
fn text_counts_characters_not_bytes() {
    let kind = text_kind(3);
    ok(kind.validate_value(&FieldValue::Text("äöü".into())));
    err_contains(kind.validate_value(&FieldValue::Text("äöüs".into())), "length");
}

#[test]
fn integer_kinds_enforce_their_access_ranges() {
    let byte = FieldKind::Number(NumericKind::Byte);
    ok(byte.validate_value(&FieldValue::Integer(0)));
    ok(byte.validate_value(&FieldValue::Integer(255)));
    err_contains(byte.validate_value(&FieldValue::Integer(256)), "range");
    err_contains(byte.validate_value(&FieldValue::Integer(-1)), "range");

    let int16 = FieldKind::Number(NumericKind::Integer);
    ok(int16.validate_value(&FieldValue::Integer(-32768)));
    ok(int16.validate_value(&FieldValue::Integer(32767)));
    err_contains(int16.validate_value(&FieldValue::Integer(32768)), "range");

    let long = FieldKind::Number(NumericKind::LongInteger);
    ok(long.validate_value(&FieldValue::Integer(i64::from(i32::MAX))));
    err_contains(
        long.validate_value(&FieldValue::Integer(i64::from(i32::MAX) + 1)),
        "range",
    );
}

#[test]
fn float_kinds_reject_non_finite_values() {
    let single = FieldKind::Number(NumericKind::Single);
    ok(single.validate_value(&FieldValue::Float(1.5)));
    err_contains(single.validate_value(&FieldValue::Float(f64::NAN)), "finite");
    err_contains(single.validate_value(&FieldValue::Float(f64::INFINITY)), "finite");
    err_contains(
        FieldKind::Number(NumericKind::Double).validate_value(&FieldValue::Float(f64::NEG_INFINITY)),
        "finite",
    );
}

#[test]
fn datetime_rejects_non_finite_and_out_of_ole_range() {
    let kind = FieldKind::DateTime;
    ok(kind.validate_value(&FieldValue::DateTime(0.0)));
    ok(kind.validate_value(&FieldValue::DateTime(-657434.0))); // 100-01-01
    ok(kind.validate_value(&FieldValue::DateTime(2958465.0))); // 9999-12-31
    err_contains(kind.validate_value(&FieldValue::DateTime(f64::NAN)), "finite");
    err_contains(
        kind.validate_value(&FieldValue::DateTime(2958466.0)),
        "range",
    );
    err_contains(kind.validate_value(&FieldValue::DateTime(-657435.0)), "range");
}

#[test]
fn autonumber_values_are_engine_assigned() {
    let kind = FieldKind::AutoNumber(AutoKind::LongInteger);
    ok(kind.validate_value(&FieldValue::Null));
    err_contains(kind.validate_value(&FieldValue::Integer(1)), "engine");
    err_contains(
        FieldKind::AutoNumber(AutoKind::ReplicationId)
            .validate_value(&FieldValue::Guid([0u8; 16])),
        "engine",
    );
}

#[test]
fn kind_mismatch_names_expected_and_found() {
    let result = FieldKind::Currency.validate_value(&FieldValue::Text("9.99".into()));
    let msg = result.expect_err("expected mismatch").to_string();
    assert!(msg.contains("currency"), "expected kind family: {msg}");
    assert!(msg.to_lowercase().contains("text"), "found value family: {msg}");
}

#[test]
fn boolean_binary_guid_currency_accept_only_their_family() {
    ok(FieldKind::Boolean { tri_state: false }.validate_value(&FieldValue::Boolean(true)));
    err_contains(
        FieldKind::Boolean { tri_state: false }.validate_value(&FieldValue::Integer(1)),
        "boolean",
    );
    ok(
        FieldKind::Binary(BinaryKind::OleObject)
            .validate_value(&FieldValue::Binary(vec![1, 2, 3])),
    );
    err_contains(
        FieldKind::Binary(BinaryKind::OleObject).validate_value(&FieldValue::Text("x".into())),
        "binary",
    );
    ok(FieldKind::Currency.validate_value(&FieldValue::Currency(-10_000)));
    err_contains(
        FieldKind::Currency.validate_value(&FieldValue::Float(1.0)),
        "currency",
    );
    ok(
        FieldKind::Number(NumericKind::Decimal {
            precision: 5,
            scale: 2,
        })
        .validate_value(&FieldValue::Decimal(-12345)),
    );
}

// ---------------------------------------------------------------------------
// field definitions
// ---------------------------------------------------------------------------

#[test]
fn field_def_rejects_required_null_and_disallowed_empty_string() {
    let mut field = FieldDef::new("name", text_kind(10));
    ok(field.validate_value(&FieldValue::Null));

    field.properties.required = true;
    err_contains(field.validate_value(&FieldValue::Null), "required");
    ok(field.validate_value(&FieldValue::Text("x".into())));

    let mut strict = FieldDef::new("note", FieldKind::Memo);
    strict.properties.required = true;
    strict.properties.allow_zero_length = true;
    err_contains(strict.validate_value(&FieldValue::Text("".into())), "empty");
    ok(strict.validate_value(&FieldValue::Text("note".into())));
}

#[test]
fn field_def_validates_name_and_kind_and_properties() {
    let mut field = FieldDef::new("", text_kind(5));
    err_contains(field.validate(), "name");

    let mut field = FieldDef::new("amount", FieldKind::Number(NumericKind::Byte));
    field.properties.allow_zero_length = true;
    err_contains(field.validate(), "allow_zero_length");
}

// ---------------------------------------------------------------------------
// table definitions
// ---------------------------------------------------------------------------

#[test]
fn table_def_rejects_duplicate_field_names_case_insensitively() {
    let table = TableDef::new(
        "customers",
        vec![
            FieldDef::new("City", text_kind(50)),
            FieldDef::new("city", text_kind(50)),
        ],
    );
    err_contains(table.validate(), "duplicate");

    let table = TableDef::new("customers", vec![FieldDef::new("City", text_kind(50))]);
    ok(table.validate());
}

#[test]
fn table_def_rejects_empty_table_name() {
    let table = TableDef::new("", vec![FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger))]);
    err_contains(table.validate(), "name");
}

// ---------------------------------------------------------------------------
// serde round-trips (the step 3.2 document builds on this)
// ---------------------------------------------------------------------------

#[test]
fn field_kinds_survive_json_round_trip() {
    let kinds = vec![
        text_kind(255),
        FieldKind::Memo,
        FieldKind::Number(NumericKind::Decimal {
            precision: 28,
            scale: 8,
        }),
        FieldKind::AutoNumber(AutoKind::ReplicationId),
        FieldKind::Boolean { tri_state: true },
        FieldKind::Binary(BinaryKind::Attachment),
    ];
    let json = serde_json::to_string(&kinds).expect("serialize kinds");
    let back: Vec<FieldKind> = serde_json::from_str(&json).expect("deserialize kinds");
    assert_eq!(back, kinds);
}

#[test]
fn field_values_survive_json_round_trip() {
    let values = vec![
        FieldValue::Null,
        FieldValue::Boolean(true),
        FieldValue::Integer(-42),
        FieldValue::Float(1.5),
        FieldValue::Currency(-123456),
        FieldValue::DateTime(45123.25),
        FieldValue::Decimal(-987654321),
        FieldValue::Text("äöü ✓".into()),
        FieldValue::Binary(vec![0, 255, 7]),
        FieldValue::Guid([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
    ];
    let json = serde_json::to_string(&values).expect("serialize values");
    let back: Vec<FieldValue> = serde_json::from_str(&json).expect("deserialize values");
    assert_eq!(back, values);
}

// ---------------------------------------------------------------------------
// property-based checks
// ---------------------------------------------------------------------------

proptest! {
    /// Any text value with at most `max_len` unicode scalar values passes;
    /// one more character fails — regardless of the byte length.
    #[test]
    fn text_validation_counts_scalars(max_len in 1u16..=255, extra in 0u16..=3) {
        let kind = text_kind(max_len);
        let len = max_len as usize;
        let at_limit: String = "ä".repeat(len);
        let over_limit: String = "ä".repeat(len + extra as usize + 1);
        prop_assert!(kind.validate_value(&FieldValue::Text(at_limit)).is_ok());
        prop_assert!(kind.validate_value(&FieldValue::Text(over_limit)).is_err());
    }

    /// Every representable 32-bit integer survives the JSON round-trip.
    #[test]
    fn integer_values_json_roundtrip(v in any::<i32>()) {
        let value = FieldValue::Integer(i64::from(v));
        let json = serde_json::to_string(&value).unwrap();
        let back: FieldValue = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, value);
    }

    /// Long-integers within range always validate; out of range never does.
    #[test]
    fn long_integer_range_is_enforced(v in any::<i64>()) {
        let kind = FieldKind::Number(NumericKind::LongInteger);
        let result = kind.validate_value(&FieldValue::Integer(v));
        prop_assert_eq!(result.is_ok(), i64::from(i32::MIN) <= v && v <= i64::from(i32::MAX));
    }
}

#[test]
fn index_mode_is_part_of_the_property_model() {
    let mut props = FieldProperties::default();
    props.indexed = IndexMode::NoDuplicates;
    ok(props.validate_for(&text_kind(10)));
    assert_eq!(props.indexed, IndexMode::NoDuplicates);
    assert_ne!(props.indexed, IndexMode::DuplicatesOk);
    assert_ne!(props.indexed, IndexMode::None);
}
