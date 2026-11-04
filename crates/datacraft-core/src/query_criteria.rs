//! Criteria parsing (step 7.3, item H27).
//!
//! Criteria strings parse to typed predicates per column kind: optional
//! operator prefix, then a literal coerced to the column family. Family
//! comparisons and OLE date math live here; execution is
//! [crate::query_engine].

use crate::query_engine::EngineError;
use crate::schema::FieldValue;
use crate::schema::{FieldDef, FieldKind, NumericKind};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Predicate {
    Equal(FieldValue),
    NotEqual(FieldValue),
    Less(FieldValue),
    LessEqual(FieldValue),
    Greater(FieldValue),
    GreaterEqual(FieldValue),
}

impl Predicate {
    /// Evaluates against a row value (NULL never satisfies). Complex values
    /// use EXISTS semantics: the predicate holds when at least one
    /// sub-record's scalar satisfies it (step 16.1, M4).
    pub(crate) fn matches(&self, value: &FieldValue) -> bool {
        let literal = match self {
            Predicate::Equal(v)
            | Predicate::NotEqual(v)
            | Predicate::Less(v)
            | Predicate::LessEqual(v)
            | Predicate::Greater(v)
            | Predicate::GreaterEqual(v) => v,
        };
        if let FieldValue::Complex(records) = value {
            return records
                .iter()
                .filter_map(|record| record.scalar())
                .any(|scalar| self.matches_scalar(scalar, literal));
        }
        self.matches_scalar(value, literal)
    }

    /// Scalar comparison shared by plain and complex cells.
    fn matches_scalar(&self, value: &FieldValue, literal: &FieldValue) -> bool {
        if matches!(value, FieldValue::Null | FieldValue::Guid(_)) {
            return false;
        }
        let Some(ordering) = compare(value, literal) else {
            return false;
        };
        match self {
            Predicate::Equal(_) => ordering == std::cmp::Ordering::Equal,
            Predicate::NotEqual(_) => ordering != std::cmp::Ordering::Equal,
            Predicate::Less(_) => ordering == std::cmp::Ordering::Less,
            Predicate::LessEqual(_) => ordering != std::cmp::Ordering::Greater,
            Predicate::Greater(_) => ordering == std::cmp::Ordering::Greater,
            Predicate::GreaterEqual(_) => ordering != std::cmp::Ordering::Less,
        }
    }
}

/// Family-safe comparison; `None` on kind mismatch or non-finite floats.
pub(crate) fn compare(a: &FieldValue, b: &FieldValue) -> Option<std::cmp::Ordering> {
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

/// literal per the column kind. A plain value means equality.
pub(crate) fn parse_predicate(
    text: &str,
    field: &str,
    column: &FieldDef,
) -> Result<Predicate, EngineError> {
    let trimmed = text.trim();
    let (operator, literal) = if let Some(rest) = trimmed.strip_prefix("<>") {
        ("<>", rest)
    } else if let Some(rest) = trimmed.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = trimmed.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = trimmed.strip_prefix('=') {
        ("=", rest)
    } else if let Some(rest) = trimmed.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = trimmed.strip_prefix('>') {
        (">", rest)
    } else {
        ("=", trimmed)
    };
    let value = coerce(literal.trim(), &column.kind).map_err(|detail| EngineError::BadLiteral {
        field: field.to_owned(),
        criteria: text.to_owned(),
        detail,
    })?;
    Ok(match operator {
        "=" => Predicate::Equal(value),
        "<>" => Predicate::NotEqual(value),
        "<" => Predicate::Less(value),
        "<=" => Predicate::LessEqual(value),
        ">" => Predicate::Greater(value),
        _ => Predicate::GreaterEqual(value),
    })
}

/// Coerces a literal to the column kind (Access-like wording accepted).
fn coerce(literal: &str, kind: &FieldKind) -> Result<FieldValue, String> {
    let unquoted = literal
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(literal);
    match kind {
        FieldKind::Number(NumericKind::Byte)
        | FieldKind::Number(NumericKind::Integer)
        | FieldKind::Number(NumericKind::LongInteger)
        | FieldKind::AutoNumber(crate::schema::AutoKind::LongInteger) => unquoted
            .parse::<i64>()
            .map(FieldValue::Integer)
            .map_err(|e| format!("{unquoted:?} is not an integer ({e})")),
        FieldKind::Number(NumericKind::Single) | FieldKind::Number(NumericKind::Double) => {
            let value: f64 = unquoted
                .parse()
                .map_err(|e| format!("{unquoted:?} is not a number ({e})"))?;
            if value.is_finite() {
                Ok(FieldValue::Float(value))
            } else {
                Err(format!("{unquoted:?} is not finite"))
            }
        }
        FieldKind::Number(NumericKind::Decimal { .. }) => unquoted
            .parse::<i128>()
            .map(FieldValue::Decimal)
            .map_err(|e| format!("{unquoted:?} is not a scaled integer ({e})")),
        FieldKind::Currency => {
            let value: f64 = unquoted
                .parse()
                .map_err(|e| format!("{unquoted:?} is not a currency amount ({e})"))?;
            Ok(FieldValue::Currency((value * 10_000.0).round() as i64))
        }
        FieldKind::DateTime => parse_date(unquoted)
            .ok_or_else(|| format!("{unquoted:?} is not a YYYY-MM-DD [HH:MM] date")),
        FieldKind::Boolean { .. } => match unquoted.to_lowercase().as_str() {
            "yes" | "true" | "1" => Ok(FieldValue::Boolean(true)),
            "no" | "false" | "0" => Ok(FieldValue::Boolean(false)),
            other => Err(format!("{other:?} is not a yes/no value")),
        },
        FieldKind::Text { .. } | FieldKind::Memo => Ok(FieldValue::Text(unquoted.to_owned())),
        // Multi-value criteria coerce to the element family; the predicate
        // then applies with EXISTS semantics over the sub-records.
        FieldKind::MultiValue(multi) => coerce(literal, &multi.element_kind()),
        FieldKind::AutoNumber(crate::schema::AutoKind::ReplicationId)
        | FieldKind::Guid
        | FieldKind::Binary(_)
        | FieldKind::Complex => Err(format!(
            "criteria are not supported on {:?} columns",
            crate::present::field_kind_display(kind)
        )),
    }
}

/// Parses `YYYY-MM-DD` (optionally ` HH:MM`) into OLE Automation days;
/// shared with the expression service's date literals.
pub(crate) fn parse_date(text: &str) -> Option<FieldValue> {
    let (date, time) = match text.split_once(' ') {
        Some((d, t)) => (d, Some(t)),
        None => (text, None),
    };
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut days = (days_from_civil(year, month, day) + 25_569) as f64;
    if let Some(time) = time {
        let mut parts = time.split(':');
        let hour: i64 = parts.next()?.parse().ok()?;
        let minute: i64 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(0..24).contains(&hour) || !(0..60).contains(&minute) {
            return None;
        }
        days += (hour * 60 + minute) as f64 / 1440.0;
    }
    Some(FieldValue::DateTime(days))
}

/// Days since 1970-01-01 for a civil date (inverse of civil_from_days);
/// shared with the expression service's `DateSerial`.
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AutoKind, BinaryKind, NumericKind, TableDef};

    fn kind_of(name: &str, kind: FieldKind) -> TableDef {
        TableDef::new(name, vec![FieldDef::new("v", kind)])
    }

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

    /// mismatches never satisfy.
    #[test]
    fn predicates_match_their_band() {
        use FieldValue as V;
        let cases: [(Predicate, V, bool); 7] = [
            (Predicate::Equal(V::Integer(2)), V::Integer(2), true),
            (Predicate::NotEqual(V::Integer(3)), V::Integer(2), true),
            (Predicate::Less(V::Integer(3)), V::Integer(2), true),
            (Predicate::LessEqual(V::Integer(2)), V::Integer(2), true),
            (Predicate::Greater(V::Integer(1)), V::Integer(2), true),
            (Predicate::GreaterEqual(V::Integer(2)), V::Integer(2), true),
            (Predicate::Equal(V::Integer(3)), V::Integer(2), false),
        ];
        for (predicate, value, expected) in cases {
            assert_eq!(
                predicate.matches(&value),
                expected,
                "{predicate:?} on {value:?}"
            );
        }
        assert!(!Predicate::Equal(V::Integer(2)).matches(&V::Null));
        assert!(!Predicate::Equal(V::Integer(2)).matches(&V::Text("2".into())));
    }

    /// Coercion covers every kind, including its error paths.
    #[test]
    fn coercion_covers_every_kind() {
        use NumericKind as N;
        let int = kind_of("i", FieldKind::Number(N::LongInteger));
        assert_eq!(
            coerce("42", &int.fields[0].kind).unwrap(),
            FieldValue::Integer(42)
        );
        assert!(coerce("4.2", &int.fields[0].kind).is_err());

        let dbl = kind_of("d", FieldKind::Number(N::Double));
        assert_eq!(
            coerce("1.5", &dbl.fields[0].kind).unwrap(),
            FieldValue::Float(1.5)
        );

        let dec = kind_of(
            "c",
            FieldKind::Number(N::Decimal {
                precision: 9,
                scale: 0,
            }),
        );
        assert_eq!(
            coerce("-7", &dec.fields[0].kind).unwrap(),
            FieldValue::Decimal(-7)
        );

        let cur = kind_of("u", FieldKind::Currency);
        assert_eq!(
            coerce("12.5", &cur.fields[0].kind).unwrap(),
            FieldValue::Currency(125_000)
        );
        assert!(coerce("x", &cur.fields[0].kind).is_err());

        let dt = kind_of("t", FieldKind::DateTime);
        assert!(coerce("2025-01-01 10:30", &dt.fields[0].kind).is_ok());
        assert!(coerce("2025-13-01", &dt.fields[0].kind).is_err());
        assert!(coerce("2025-01-01 25:00", &dt.fields[0].kind).is_err());
        assert!(coerce("nope", &dt.fields[0].kind).is_err());

        let boolean = kind_of("b", FieldKind::Boolean { tri_state: false });
        assert_eq!(
            coerce("No", &boolean.fields[0].kind).unwrap(),
            FieldValue::Boolean(false)
        );
        assert!(coerce("maybe", &boolean.fields[0].kind).is_err());

        let text = kind_of("s", FieldKind::Text { max_len: 10 });
        assert_eq!(
            coerce("\"Ada\"", &text.fields[0].kind).unwrap(),
            FieldValue::Text("Ada".into())
        );

        let guid = kind_of("g", FieldKind::Guid);
        assert!(coerce("x", &guid.fields[0].kind).is_err());
        let ole = kind_of("o", FieldKind::Binary(BinaryKind::OleObject));
        assert!(coerce("x", &ole.fields[0].kind).is_err());
        let auto = kind_of("a", FieldKind::AutoNumber(AutoKind::ReplicationId));
        assert!(coerce("x", &auto.fields[0].kind).is_err());
    }

    /// Step 39.3 (item M37): `Like` criteria work on text columns through
    /// the expression matcher (Access wildcards, case-insensitive) instead
    /// of degrading to an equality against the literal criteria text.
    #[test]
    fn like_criteria_match_access_patterns() {
        let text = kind_of("s", FieldKind::Text { max_len: 50 });
        let column = &text.fields[0];
        let predicate = parse_predicate("Like \"B*\"", "v", column).expect("Like parses");
        assert!(
            predicate.matches(&FieldValue::Text("Berlin".into())),
            "B* matches Berlin"
        );
        assert!(!predicate.matches(&FieldValue::Text("Hamburg".into())));
        let predicate =
            parse_predicate("like \"hamburg\"", "v", column).expect("like parses");
        assert!(
            predicate.matches(&FieldValue::Text("Hamburg".into())),
            "the keyword and the pattern are case-insensitive"
        );
    }

    /// Step 39.3 (item M37): the other Access criteria forms are recognized
    /// and answer a precise error — on text columns they previously
    /// degraded to an equality against the literal criteria text (silent
    /// 0 rows).
    #[test]
    fn unsupported_access_forms_error_precisely() {
        let text = kind_of("s", FieldKind::Text { max_len: 50 });
        let column = &text.fields[0];
        for criteria in ["Between 1 And 5", "In (1, 2)", "Is Null", "Not Like \"B*\""] {
            let error = parse_predicate(criteria, "v", column)
                .expect_err("recognized Access forms never degrade to equality");
            assert!(error.to_string().contains(criteria), "{criteria}: {error}");
        }
    }

    /// Step 39.3 (item M37): Like is a text-column form — other families
    /// answer a precise reason instead of a coercion failure.
    #[test]
    fn like_on_non_text_columns_error_precisely() {
        let numbers = kind_of("i", FieldKind::Number(NumericKind::LongInteger));
        let error = parse_predicate("Like \"B*\"", "v", &numbers.fields[0])
            .expect_err("Like needs a text column");
        assert!(error.to_string().contains("text"), "{error}");
    }

    /// Step 39.3 (item M37): the case fence — equality criteria compare
    /// case-sensitively (byte order), Like matches case-insensitively
    /// (Option Compare Database via the expression matcher).
    #[test]
    fn text_criteria_case_fence_is_pinned() {
        let text = kind_of("s", FieldKind::Text { max_len: 50 });
        let column = &text.fields[0];
        let equal = parse_predicate("berlin", "v", column).expect("parses");
        assert!(
            !equal.matches(&FieldValue::Text("Berlin".into())),
            "equality is case-sensitive"
        );
        let like = parse_predicate("Like \"berlin\"", "v", column).expect("parses");
        assert!(
            like.matches(&FieldValue::Text("Berlin".into())),
            "Like is case-insensitive"
        );
    }
}
