//! Report band aggregates (step 15.2, item H41).
//!
//! The `=Sum/Count/Min/Max/Avg(…)` semantics of the report footer bands:
//! the argument evaluates per row (Nulls are skipped), `Count(*)` counts
//! rows, `Sum` keeps currency on its scale, `Avg` widens to float, and
//! `Min`/`Max` order through the expression service's family rules.

use crate::expr::{self, Scope};
use crate::schema::{FieldValue, TableDef};

/// Builds the expression scope of one row (qualified entries; bare
/// lookups resolve through the qualified ones).
pub(crate) fn scope_of(source: Option<&TableDef>, row: &[FieldValue]) -> Scope {
    let mut scope = Scope::new();
    if let Some(source) = source {
        for (field, value) in source.fields.iter().zip(row.iter()) {
            scope.set_field(source.name.clone(), field.name.clone(), value.clone());
        }
    }
    scope
}

/// One footer aggregate function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Aggregate {
    Sum,
    Count,
    Min,
    Max,
    Avg,
}

/// `Sum/Count/Min/Max/Avg(…)` at the top level (case-insensitive).
pub(crate) fn aggregate_head(expression: &str) -> Option<(Aggregate, String)> {
    let trimmed = expression.trim();
    let (head, rest) = trimmed.split_once('(')?;
    let aggregate = match head.trim().to_lowercase().as_str() {
        "sum" => Aggregate::Sum,
        "count" => Aggregate::Count,
        "min" => Aggregate::Min,
        "max" => Aggregate::Max,
        "avg" => Aggregate::Avg,
        _ => return None,
    };
    let argument = rest.strip_suffix(')')?;
    Some((aggregate, argument.to_owned()))
}

pub(crate) fn run_aggregate(
    aggregate: Aggregate,
    argument: &str,
    source: Option<&TableDef>,
    rows: &[&Vec<FieldValue>],
) -> Result<FieldValue, String> {
    if matches!(aggregate, Aggregate::Count) && argument.trim() == "*" {
        return Ok(FieldValue::Integer(rows.len() as i64));
    }
    let parsed = expr::parse(argument).map_err(|e| e.to_string())?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let scope = scope_of(source, row);
        let value = parsed
            .evaluate(&scope)
            .map_err(|e| format!("aggregate {argument:?}: {e}"))?;
        if !matches!(value, FieldValue::Null) {
            values.push(value);
        }
    }
    Ok(match aggregate {
        Aggregate::Count => FieldValue::Integer(values.len() as i64),
        Aggregate::Min | Aggregate::Max => {
            let mut best: Option<FieldValue> = None;
            for value in &values {
                let take = match &best {
                    None => true,
                    Some(current) => {
                        let ordering =
                            expr::field_ord(current, value).map_err(|e| e.to_string())?;
                        matches!(
                            (ordering, aggregate),
                            (std::cmp::Ordering::Greater, Aggregate::Min)
                                | (std::cmp::Ordering::Less, Aggregate::Max)
                        )
                    }
                };
                if take {
                    best = Some(value.clone());
                }
            }
            best.unwrap_or(FieldValue::Null)
        }
        Aggregate::Sum | Aggregate::Avg => sum_values(&values, aggregate == Aggregate::Avg)?,
    })
}

/// Sums non-Null values; currency stays currency, everything else widens
/// to float (`Avg` divides by the count).
fn sum_values(values: &[FieldValue], average: bool) -> Result<FieldValue, String> {
    if values.is_empty() {
        return Ok(FieldValue::Null);
    }
    let all_currency = values
        .iter()
        .all(|value| matches!(value, FieldValue::Currency(_)));
    if all_currency && !average {
        let mut scaled: i64 = 0;
        for value in values {
            let FieldValue::Currency(amount) = value else {
                unreachable!("checked above")
            };
            scaled = scaled
                .checked_add(*amount)
                .ok_or_else(|| "currency sum overflows".to_owned())?;
        }
        return Ok(FieldValue::Currency(scaled));
    }
    let mut total = 0.0;
    for value in values {
        total += expr::numeric_view(value).map_err(|e| e.to_string())?;
    }
    if average {
        total /= values.len() as f64;
    }
    Ok(FieldValue::Float(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, FieldKind, NumericKind};

    fn table() -> TableDef {
        TableDef::new(
            "T",
            vec![
                FieldDef::new("region", FieldKind::Text { max_len: 10 }),
                FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("price", FieldKind::Currency),
            ],
        )
    }

    fn group() -> Vec<Vec<FieldValue>> {
        vec![
            vec![
                FieldValue::Text("West".into()),
                FieldValue::Integer(2),
                FieldValue::Currency(10_000),
            ],
            vec![
                FieldValue::Text("West".into()),
                FieldValue::Integer(1),
                FieldValue::Currency(20_000),
            ],
        ]
    }

    #[test]
    fn aggregates_cover_every_family() {
        let data = group();
        let rows: Vec<&Vec<FieldValue>> = data.iter().collect();
        assert_eq!(
            run_aggregate(Aggregate::Sum, "[price]", Some(&table()), &rows).expect("sums"),
            FieldValue::Currency(30_000)
        );
        assert_eq!(
            run_aggregate(Aggregate::Avg, "[price]", Some(&table()), &rows).expect("averages"),
            FieldValue::Float(1.5)
        );
        assert_eq!(
            run_aggregate(Aggregate::Min, "[qty]", Some(&table()), &rows).expect("mins"),
            FieldValue::Integer(1)
        );
        assert_eq!(
            run_aggregate(Aggregate::Max, "[qty]", Some(&table()), &rows).expect("maxes"),
            FieldValue::Integer(2)
        );
        assert_eq!(
            run_aggregate(Aggregate::Count, "[qty]", Some(&table()), &rows).expect("counts"),
            FieldValue::Integer(2)
        );
        assert_eq!(
            run_aggregate(Aggregate::Count, "*", Some(&table()), &rows).expect("count(*)"),
            FieldValue::Integer(2)
        );
    }

    #[test]
    fn null_values_are_skipped_by_aggregates() {
        let data = [vec![
            FieldValue::Text("W".into()),
            FieldValue::Null,
            FieldValue::Currency(10_000),
        ]];
        let rows: Vec<&Vec<FieldValue>> = data.iter().collect();
        assert_eq!(
            run_aggregate(Aggregate::Sum, "[qty]", Some(&table()), &rows).expect("skips null"),
            FieldValue::Null
        );
    }

    #[test]
    fn unknown_aggregate_fields_fail_precisely() {
        let data = group();
        let rows: Vec<&Vec<FieldValue>> = data.iter().collect();
        let error = run_aggregate(Aggregate::Sum, "[ghost]", Some(&table()), &rows)
            .expect_err("unknown field must fail");
        assert!(error.contains("ghost"), "{error}");
    }
}
