//! Scale-aware decimal display (phase 104, step 104.1; item M135).
//!
//! A fixed-point decimal is a scale-scaled integer, but the bare
//! `display_value` prints the raw scaled integer. The kind-aware
//! sibling divides by the declared scale and keeps the trailing zeros;
//! scale 0 stays byte-identical, and a kind-less context keeps the raw
//! form (nothing is known about the column there).

use datacraft_types::{FieldKind, FieldValue, NumericKind, display_value, display_value_for};

fn decimal(precision: u8, scale: u8) -> Option<FieldKind> {
    Some(FieldKind::Number(NumericKind::Decimal { precision, scale }))
}

#[test]
fn a_scale_two_decimal_renders_its_declared_scale() {
    assert_eq!(
        display_value_for(&FieldValue::Decimal(2550), decimal(18, 2).as_ref()),
        "25.50"
    );
    assert_eq!(
        display_value_for(&FieldValue::Decimal(255), decimal(10, 2).as_ref()),
        "2.55"
    );
    assert_eq!(
        display_value_for(&FieldValue::Decimal(-5), decimal(10, 2).as_ref()),
        "-0.05"
    );
    assert_eq!(
        display_value_for(&FieldValue::Decimal(-12_345_678), decimal(18, 4).as_ref()),
        "-1234.5678"
    );
}

#[test]
fn scale_zero_stays_byte_identical_to_the_raw_form() {
    assert_eq!(
        display_value_for(&FieldValue::Decimal(7), decimal(10, 0).as_ref()),
        "7"
    );
    assert_eq!(
        display_value_for(&FieldValue::Decimal(2550), decimal(18, 0).as_ref()),
        "2550"
    );
}

#[test]
fn a_kind_less_context_keeps_the_raw_scaled_integer() {
    assert_eq!(display_value_for(&FieldValue::Decimal(2550), None), "2550");
    // The bare helper is unchanged — scale-agnostic contexts (computed
    // values without a column) keep today's rendering.
    assert_eq!(display_value(&FieldValue::Decimal(2550)), "2550");
}

#[test]
fn every_other_family_is_kind_agnostic() {
    for (value, kind) in [
        (FieldValue::Integer(42), decimal(10, 2).as_ref()),
        (FieldValue::Text("Ada".into()), None),
        (FieldValue::Currency(12_500), None),
        (FieldValue::Boolean(true), None),
        (FieldValue::Null, None),
    ] {
        assert_eq!(display_value_for(&value, kind), display_value(&value));
    }
}
