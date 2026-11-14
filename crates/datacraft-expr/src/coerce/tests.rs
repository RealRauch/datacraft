//! Coercion edge-path pins (step 43.1, item T9): the numeric tower,
//! condition views, negation and cross-family ordering.

use super::*;

#[test]
fn to_number_covers_every_coercible_family() {
    assert_eq!(to_number(&FieldValue::Boolean(true)), Ok(Num::Int(-1)));
    assert_eq!(to_number(&FieldValue::Integer(7)), Ok(Num::Int(7)));
    assert_eq!(to_number(&FieldValue::Currency(5)), Ok(Num::Currency(5)));
    assert_eq!(to_number(&FieldValue::Float(1.5)), Ok(Num::Float(1.5)));
    assert_eq!(to_number(&FieldValue::DateTime(2.0)), Ok(Num::Float(2.0)));
    assert_eq!(to_number(&FieldValue::Decimal(9)), Ok(Num::Decimal(9)));
    assert_eq!(to_number(&FieldValue::Text(" 4 ".into())), Ok(Num::Int(4)));
    assert_eq!(
        to_number(&FieldValue::Text("4.5".into())),
        Ok(Num::Float(4.5))
    );
    assert!(to_number(&FieldValue::Text("x".into())).is_err());
    assert!(to_number(&FieldValue::Binary(vec![])).is_err());
}

#[test]
fn vba_round_goes_to_the_even_half() {
    assert_eq!(vba_round(2.5), 2);
    assert_eq!(vba_round(3.5), 4);
    assert_eq!(vba_round(-2.5), -2);
    assert_eq!(vba_round(7.4), 7);
}

#[test]
fn bool_state_reads_all_condition_sources() {
    assert_eq!(bool_state(&FieldValue::Null), Ok(None));
    assert_eq!(bool_state(&FieldValue::Boolean(false)), Ok(Some(false)));
    assert_eq!(bool_state(&FieldValue::Integer(3)), Ok(Some(true)));
    assert_eq!(bool_state(&FieldValue::Text("Yes".into())), Ok(Some(true)));
    assert_eq!(bool_state(&FieldValue::Text("2".into())), Ok(Some(true)));
    assert!(bool_state(&FieldValue::Text("maybe".into())).is_err());
    assert!(bool_state(&FieldValue::Guid([0; 16])).is_err());
}

#[test]
fn currency_multiply_stays_on_its_scale() {
    assert_eq!(
        arith(
            Arithmetic::Multiply,
            &FieldValue::Currency(125_000),
            &FieldValue::Integer(2),
        ),
        Ok(FieldValue::Currency(250_000))
    );
}

#[test]
fn arithmetic_propagates_null_before_coercing() {
    assert_eq!(
        arith(
            Arithmetic::Add,
            &FieldValue::Null,
            &FieldValue::Text("a".into())
        ),
        Ok(FieldValue::Null)
    );
}

#[test]
fn negation_follows_the_tower() {
    assert_eq!(
        negate(&FieldValue::Boolean(true)),
        Ok(FieldValue::Integer(1))
    );
    assert_eq!(negate(&FieldValue::Null), Ok(FieldValue::Null));
    assert_eq!(
        negate(&FieldValue::Text("2.5".into())),
        Ok(FieldValue::Float(-2.5))
    );
    assert!(negate(&FieldValue::Guid([0; 16])).is_err());
}

#[test]
fn ordering_compares_cross_family_numerics_as_float() {
    assert_eq!(
        ord_cmp(&FieldValue::Currency(15_000), &FieldValue::Float(1.2)),
        Ok(std::cmp::Ordering::Greater)
    );
    assert_eq!(
        ord_cmp(&FieldValue::Text("2.5".into()), &FieldValue::Integer(3)),
        Ok(std::cmp::Ordering::Less)
    );
    assert!(ord_cmp(&FieldValue::Binary(vec![]), &FieldValue::Integer(1)).is_err());
}

#[test]
fn display_renders_null_as_empty_text() {
    assert_eq!(display(&FieldValue::Null), "");
    assert_eq!(display(&FieldValue::Text("a".into())), "a");
    assert_eq!(display(&FieldValue::Integer(3)), "3");
}

#[test]
fn non_zero_reads_every_numeric_family() {
    assert!(Num::Int(1).non_zero());
    assert!(!Num::Int(0).non_zero());
    assert!(Num::Currency(-1).non_zero());
    assert!(!Num::Currency(0).non_zero());
    assert!(Num::Float(0.5).non_zero());
    assert!(!Num::Float(0.0).non_zero());
    assert!(Num::Decimal(1).non_zero());
    assert!(!Num::Decimal(0).non_zero());
}

#[test]
fn condition_view_covers_the_numeric_families() {
    assert_eq!(bool_state(&FieldValue::Currency(-2)), Ok(Some(true)));
    assert_eq!(bool_state(&FieldValue::Currency(0)), Ok(Some(false)));
    assert_eq!(bool_state(&FieldValue::Decimal(7)), Ok(Some(true)));
    assert_eq!(bool_state(&FieldValue::Float(0.25)), Ok(Some(true)));
    assert!(bool_state(&FieldValue::Float(f64::INFINITY)).is_err());
    assert!(bool_state(&FieldValue::DateTime(f64::NAN)).is_err());
}

#[test]
fn negation_covers_decimals_and_numeric_text() {
    assert_eq!(negate(&FieldValue::Decimal(-9)), Ok(FieldValue::Decimal(9)));
    assert_eq!(
        negate(&FieldValue::Text("-2".into())),
        Ok(FieldValue::Integer(2))
    );
}

#[test]
fn mixed_decimal_float_arithmetic_widens_to_float() {
    // The generic float arm is the only place a Decimal operand
    // enters the f64 view (decimals pair with themselves or ints
    // otherwise).
    assert_eq!(
        arith(
            Arithmetic::Add,
            &FieldValue::Decimal(9),
            &FieldValue::Float(0.5),
        ),
        Ok(FieldValue::Float(9.5))
    );
}

#[test]
fn ordering_resolves_either_side_being_numeric_text() {
    assert_eq!(
        ord_cmp(&FieldValue::Integer(3), &FieldValue::Text("2.5".into())),
        Ok(std::cmp::Ordering::Greater)
    );
}
