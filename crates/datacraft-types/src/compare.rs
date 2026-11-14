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
}
