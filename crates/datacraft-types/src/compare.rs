//! Family-safe value ordering (step 7.3, item H27).
//!
//! The expression service's comparison rules, the criteria predicates, the
//! table view's sort and the native-execution join/order paths share one
//! ordering definition: values of the same family compare, everything else
//! (kind mismatch, non-finite floats) answers `None`.

use crate::FieldValue;

/// Family-safe comparison; `None` on kind mismatch or non-finite floats.
pub fn compare(a: &FieldValue, b: &FieldValue) -> Option<std::cmp::Ordering> {
    use FieldValue as V;
    match (a, b) {
        (V::Boolean(x), V::Boolean(y)) => Some(x.cmp(y)),
        (V::Integer(x), V::Integer(y)) => Some(x.cmp(y)),
        (V::Float(x), V::Float(y)) if x.is_finite() && y.is_finite() => x.partial_cmp(y),
        (V::Currency(x), V::Currency(y)) => Some(x.cmp(y)),
        (V::DateTime(x), V::DateTime(y)) if x.is_finite() && y.is_finite() => x.partial_cmp(y),
        (V::Decimal(x), V::Decimal(y)) => Some(x.cmp(y)),
        (V::Text(x), V::Text(y)) => Some(x.cmp(y)),
        (V::Binary(x), V::Binary(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// The total sort comparator (step 91.5, item M113) — defined over every
/// value pair, so a sort can never scramble:
///
/// - NULL placement follows the Access convention: NULLs first ascending,
///   and the direction's reverse puts them last descending;
/// - comparable pairs answer [`compare`];
/// - mixed families fall to the documented family rank (the `FieldValue`
///   declaration order), and non-finite floats tier after every
///   comparable value of their family (ties keep the stable order).
///
/// [`compare`] itself stays family-safe `Option` for its other callers.
pub fn sort_compare(a: &FieldValue, b: &FieldValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (FieldValue::Null, FieldValue::Null) => Ordering::Equal,
        (FieldValue::Null, _) => Ordering::Less,
        (_, FieldValue::Null) => Ordering::Greater,
        _ => compare(a, b).unwrap_or_else(|| sort_tier(a).cmp(&sort_tier(b))),
    }
}

/// `(family rank, tier)` of a non-NULL value: the rank orders the
/// families in [`FieldValue`] declaration order; the tier separates
/// comparable values (0) from non-finite floats (1) inside one family.
fn sort_tier(value: &FieldValue) -> (u8, u8) {
    let rank = match value {
        FieldValue::Boolean(_) => 1,
        FieldValue::Integer(_) => 2,
        FieldValue::Float(_) => 3,
        FieldValue::Currency(_) => 4,
        FieldValue::DateTime(_) => 5,
        FieldValue::Decimal(_) => 6,
        FieldValue::Text(_) => 7,
        FieldValue::Binary(_) => 8,
        FieldValue::Guid(_) => 9,
        FieldValue::Complex(_) => 10,
        FieldValue::Null => 0, // unreachable — the NULL arms match first
    };
    let comparable = compare(value, value).is_some();
    (rank, u8::from(!comparable))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Family-safe comparison arms, including the mismatch and
    /// non-finite floats.
    #[test]
    fn compare_is_family_safe() {
        use FieldValue as V;
        assert_eq!(
            compare(&V::Boolean(false), &V::Boolean(true)),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare(&V::Currency(1), &V::Currency(2)),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare(&V::DateTime(1.0), &V::DateTime(2.0)),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare(&V::Decimal(1), &V::Decimal(2)),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            compare(&V::Binary(vec![1]), &V::Binary(vec![2])),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(compare(&V::Integer(1), &V::Text("1".into())), None);
        assert_eq!(compare(&V::Float(f64::NAN), &V::Float(1.0)), None);
    }

    /// The total comparator (step 91.5, item M113): NULL placement, the
    /// family rank under mixes, and the non-finite tier.
    #[test]
    fn sort_compare_is_total_with_documented_placement() {
        use FieldValue as V;
        use std::cmp::Ordering as O;
        // NULL placement.
        assert_eq!(sort_compare(&V::Null, &V::Integer(1)), O::Less);
        assert_eq!(sort_compare(&V::Integer(1), &V::Null), O::Greater);
        assert_eq!(sort_compare(&V::Null, &V::Null), O::Equal);
        // Mixed families follow the declaration-order rank.
        assert_eq!(sort_compare(&V::Integer(1), &V::Text("1".into())), O::Less);
        assert_eq!(
            sort_compare(&V::Text("x".into()), &V::Float(1.5)),
            O::Greater
        );
        // Non-finite floats tie after the comparable values of their
        // family.
        assert_eq!(sort_compare(&V::Float(1.0), &V::Float(f64::NAN)), O::Less);
        assert_eq!(
            sort_compare(&V::Float(f64::NAN), &V::Float(f64::NAN)),
            O::Equal
        );
        // Reflexivity of the comparable values.
        assert_eq!(sort_compare(&V::Integer(3), &V::Integer(3)), O::Equal);
    }
}
