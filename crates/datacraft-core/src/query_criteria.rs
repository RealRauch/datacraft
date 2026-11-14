//! Criteria parsing (step 7.3, item H27).
//!
//! Criteria strings parse to typed predicates per column kind: optional
//! operator prefix, then a literal coerced to the column family. Family
//! comparisons and OLE date math live here; execution is
//! [crate::query_engine].

use datacraft_types::{FieldDef, FieldKind, FieldValue, NumericKind, compare, parse_date};

use crate::query_engine::EngineError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Predicate {
    Equal(FieldValue),
    NotEqual(FieldValue),
    Less(FieldValue),
    LessEqual(FieldValue),
    Greater(FieldValue),
    GreaterEqual(FieldValue),
    /// Access `Like` on a text column — matched through the expression
    /// service's Access-wildcard matcher, case-insensitively (step 39.3,
    /// item M37).
    Like(String),
}

impl Predicate {
    /// Evaluates against a row value (NULL never satisfies). Complex values
    /// use EXISTS semantics: the predicate holds when at least one
    /// sub-record's scalar satisfies it (step 16.1, M4).
    pub(crate) fn matches(&self, value: &FieldValue) -> bool {
        if let FieldValue::Complex(records) = value {
            return records
                .iter()
                .filter_map(|record| record.scalar())
                .any(|scalar| self.matches_scalar(scalar));
        }
        self.matches_scalar(value)
    }

    /// Scalar evaluation shared by plain and complex cells.
    fn matches_scalar(&self, value: &FieldValue) -> bool {
        if matches!(value, FieldValue::Null | FieldValue::Guid(_)) {
            return false;
        }
        if let Predicate::Like(pattern) = self {
            return crate::expr::like::matches(&crate::present::display_value(value), pattern);
        }
        let literal = match self {
            Predicate::Equal(v)
            | Predicate::NotEqual(v)
            | Predicate::Less(v)
            | Predicate::LessEqual(v)
            | Predicate::Greater(v)
            | Predicate::GreaterEqual(v) => v,
            Predicate::Like(_) => unreachable!("handled above"),
        };
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
            Predicate::Like(_) => unreachable!("handled above"),
        }
    }
}

/// literal per the column kind. A plain value means equality. The Access
/// forms outside the typed subset are recognized and answered precisely —
/// they never degrade to an equality fallback (step 39.3, item M37).
pub(crate) fn parse_predicate(
    text: &str,
    field: &str,
    column: &FieldDef,
) -> Result<Predicate, EngineError> {
    let trimmed = text.trim();
    let lowercase = trimmed.to_lowercase();
    if let Some(pattern) = like_pattern(trimmed, &lowercase) {
        return like_predicate(pattern, field, column);
    }
    const UNSUPPORTED_FORMS: [&str; 6] = [
        "between ",
        "in (",
        "in(",
        "is null",
        "is not null",
        "not like ",
    ];
    if UNSUPPORTED_FORMS
        .iter()
        .any(|form| lowercase.starts_with(form))
    {
        return Err(EngineError::BadLiteral {
            field: field.to_owned(),
            criteria: text.to_owned(),
            detail: "this Access criteria form is recognized but outside the \
                     supported subset (=, <>, <, <=, >, >=, Like)"
                .to_owned(),
        });
    }
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
    // A bare wildcard pattern on a text column is an implicit Like — the
    // Access filter semantics the dialog's syntax help promises (step
    // 70.1, item H58). An explicit operator keeps the literal path.
    let explicit_operator = ["<>", "<=", ">=", "=", "<", ">"]
        .iter()
        .any(|op| trimmed.starts_with(op));
    let text_column = matches!(&column.kind, FieldKind::Text { .. } | FieldKind::Memo);
    let wildcard = literal.contains(['*', '?', '#']);
    if !explicit_operator && text_column && wildcard {
        return like_predicate(literal.trim(), field, column);
    }
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

/// `Like "pattern"` with a case-insensitive keyword — the pattern argument
/// stays as written; `None` when the criteria is not a Like form.
fn like_pattern<'a>(trimmed: &'a str, lowercase: &str) -> Option<&'a str> {
    if !lowercase.starts_with("like ") {
        return None;
    }
    let argument = trimmed[5..].trim();
    let pattern = argument
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))?;
    Some(pattern)
}

/// Like is a text-column form; on text and text-element multi-value columns
/// it matches through the expression service's Access-wildcard matcher
/// (case-insensitive, `Option Compare Database` semantics).
fn like_predicate(pattern: &str, field: &str, column: &FieldDef) -> Result<Predicate, EngineError> {
    fn is_text(kind: &FieldKind) -> bool {
        matches!(kind, FieldKind::Text { .. } | FieldKind::Memo)
    }
    let supported = is_text(&column.kind)
        || matches!(&column.kind, FieldKind::MultiValue(multi) if is_text(&multi.element_kind()));
    if !supported {
        return Err(EngineError::BadLiteral {
            field: field.to_owned(),
            criteria: format!("Like \"{pattern}\""),
            detail: format!(
                "Like is only supported on text columns, not on {}",
                crate::present::field_kind_display(&column.kind)
            ),
        });
    }
    Ok(Predicate::Like(pattern.to_owned()))
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

#[cfg(test)]
mod tests;
