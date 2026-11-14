//! Expression function families (step 15.1, item H41).
//!
//! The documented built-ins: `IIf`, `Nz`, `Format`, `IsNull`, `IsNumeric`,
//! `CStr`; date functions `Year/Month/Day/Hour/Minute/DateSerial`; string
//! functions `Len/Left/Right/Mid/UCase/LCase/Trim/LTrim/RTrim/InStr/Asc/
//! Chr`; math functions `Abs/Int/Fix/Round/Sqr`. All names are
//! case-insensitive; arguments evaluate eagerly (VBA `IIf` behavior).
//! `Now`/`Date` are absent — the evaluator is deterministic.

use super::coerce::{bool_state, display, parse_num_text, runtime, to_i64};
use super::format::{format_value, ole_components};
use super::{ExprError, Expression, Scope};
use datacraft_types::FieldValue;
use datacraft_types::days_from_civil;

/// Evaluates the arguments and dispatches to the built-in.
pub(crate) fn call(
    name: &str,
    args: &[Expression],
    scope: &Scope,
) -> Result<FieldValue, ExprError> {
    let canonical = name.to_lowercase();
    let arity = arity_of(&canonical);
    let values = evaluate_args(name, args, scope)?;
    if let Some((min, max)) = arity
        && !(min..=max).contains(&values.len())
    {
        return Err(runtime(format!(
            "{name} expects {min}..={max} arguments, found {}",
            values.len()
        )));
    }
    dispatch(&canonical, name, values)
}

/// `(min, max)` argument counts per built-in; `None` for unknown names
/// (a precise runtime error after evaluation).
fn arity_of(canonical: &str) -> Option<(usize, usize)> {
    Some(match canonical {
        "iif" => (3, 3),
        "nz" => (1, 2),
        "format" => (2, 2),
        "isnull" | "isnumeric" | "cstr" | "year" | "month" | "day" | "hour" | "minute" | "len"
        | "ucase" | "lcase" | "trim" | "ltrim" | "rtrim" | "asc" | "chr" | "abs" | "int"
        | "fix" | "sqr" => (1, 1),
        "left" | "right" => (2, 2),
        "mid" => (2, 3),
        "instr" => (2, 3),
        "dateserial" => (3, 3),
        "round" => (1, 2),
        _ => return None,
    })
}

fn evaluate_args(
    name: &str,
    args: &[Expression],
    scope: &Scope,
) -> Result<Vec<FieldValue>, ExprError> {
    args.iter()
        .map(|arg| {
            arg.evaluate(scope).map_err(|e| match e {
                ExprError::Runtime { detail } => ExprError::Runtime {
                    detail: format!("{name}: {detail}"),
                },
                other => other,
            })
        })
        .collect()
}

fn dispatch(canonical: &str, name: &str, values: Vec<FieldValue>) -> Result<FieldValue, ExprError> {
    match canonical {
        "iif" => {
            let condition = bool_state(&values[0])?.unwrap_or(false);
            Ok(if condition {
                values[1].clone()
            } else {
                values[2].clone()
            })
        }
        "nz" => Ok(match &values[0] {
            FieldValue::Null => values
                .get(1)
                .cloned()
                .unwrap_or(FieldValue::Text(String::new())),
            value => value.clone(),
        }),
        "isnull" => Ok(FieldValue::Boolean(matches!(values[0], FieldValue::Null))),
        "isnumeric" => Ok(match &values[0] {
            FieldValue::Null => FieldValue::Null,
            FieldValue::Text(text) => FieldValue::Boolean(parse_num_text(text).is_ok()),
            FieldValue::Boolean(_)
            | FieldValue::Integer(_)
            | FieldValue::Currency(_)
            | FieldValue::Float(_)
            | FieldValue::DateTime(_)
            | FieldValue::Decimal(_) => FieldValue::Boolean(true),
            _ => FieldValue::Boolean(false),
        }),
        "cstr" => Ok(FieldValue::Text(match &values[0] {
            FieldValue::Null => String::new(),
            // VBA conversion semantics ("True"), unlike the display
            // layer's Yes/No used by `&`.
            FieldValue::Boolean(true) => "True".to_owned(),
            FieldValue::Boolean(false) => "False".to_owned(),
            other => display(other),
        })),
        "format" => format_value(&values[0], &values[1], name),
        "year" | "month" | "day" | "hour" | "minute" => date_component(canonical, &values[0], name),
        "dateserial" => date_serial(&values, name),
        "len" => Ok(match &values[0] {
            FieldValue::Null => FieldValue::Null,
            value => FieldValue::Integer(display(value).chars().count() as i64),
        }),
        "left" | "right" => take_side(canonical, &values, name),
        "mid" => mid(&values, name),
        "ucase" | "lcase" | "trim" | "ltrim" | "rtrim" => case_or_trim(canonical, &values[0]),
        "instr" => instr(&values, name),
        "asc" => Ok(match &values[0] {
            FieldValue::Null => FieldValue::Null,
            value => {
                let code = display(value)
                    .chars()
                    .next()
                    .ok_or_else(|| runtime(format!("{name} expects a non-empty text value")))?
                    as u32;
                FieldValue::Integer(i64::from(code))
            }
        }),
        "chr" => Ok(match &values[0] {
            FieldValue::Null => FieldValue::Null,
            value => {
                let code = to_i64(value)?;
                let character = u32::try_from(code)
                    .ok()
                    .and_then(char::from_u32)
                    .filter(|c| *c <= '\u{FFFF}')
                    .ok_or_else(|| {
                        runtime(format!(
                            "{name} expects a code point below 65536, found {code}"
                        ))
                    })?;
                FieldValue::Text(character.to_string())
            }
        }),
        "abs" => abs(&values[0]),
        "int" | "fix" => floor_or_trunc(canonical, &values[0]),
        "round" => round(&values, name),
        "sqr" => sqr(&values[0], name),
        _ => Err(runtime(format!("unknown function {name}"))),
    }
}

/// The text view of a string-family argument (Null propagates; other
/// values render through the shared presentation layer).
fn text_arg(value: &FieldValue) -> Option<String> {
    match value {
        FieldValue::Null => None,
        other => Some(display(other)),
    }
}

fn date_component(
    component: &str,
    value: &FieldValue,
    name: &str,
) -> Result<FieldValue, ExprError> {
    Ok(match value {
        FieldValue::Null => FieldValue::Null,
        value => {
            let days = match value {
                FieldValue::DateTime(days) => *days,
                FieldValue::Float(days) => *days,
                other => {
                    return Err(runtime(format!(
                        "{name} expects a date value, found {}",
                        other.family()
                    )));
                }
            };
            if !days.is_finite() {
                return Err(runtime(format!("{name} expects a finite date value")));
            }
            let (year, month, day, hour, minute) = ole_components(days);
            FieldValue::Integer(match component {
                "year" => i64::from(year),
                "month" => i64::from(month),
                "day" => i64::from(day),
                "hour" => i64::from(hour),
                _ => i64::from(minute),
            })
        }
    })
}

/// `DateSerial(y, m, d)` normalizes month/day overflow like Access.
fn date_serial(values: &[FieldValue], name: &str) -> Result<FieldValue, ExprError> {
    let mut parts = [0i64; 3];
    for (index, value) in values.iter().enumerate() {
        parts[index] = match value {
            FieldValue::Null => return Ok(FieldValue::Null),
            other => to_i64(other)?,
        };
    }
    let [year, month, day] = parts;
    let year = year + (month - 1).div_euclid(12);
    let month = (month - 1).rem_euclid(12) + 1;
    let days = days_from_civil(year, month, 1) + (day - 1) + 25_569;
    if !(-657_434.0..2_958_466.0).contains(&(days as f64)) {
        return Err(runtime(format!(
            "{name} produces {days}, outside the Access date range"
        )));
    }
    Ok(FieldValue::DateTime(days as f64))
}

fn take_side(canonical: &str, values: &[FieldValue], name: &str) -> Result<FieldValue, ExprError> {
    let Some(text) = text_arg(&values[0]) else {
        return Ok(FieldValue::Null);
    };
    let count = match &values[1] {
        FieldValue::Null => return Ok(FieldValue::Null),
        other => to_i64(other)?,
    };
    if count < 0 {
        return Err(runtime(format!("{name} expects a non-negative count")));
    }
    let chars: Vec<char> = text.chars().collect();
    let taken = if canonical == "left" {
        chars[..(count as usize).min(chars.len())].iter().collect()
    } else {
        let skip = chars.len().saturating_sub(count as usize);
        chars[skip..].iter().collect()
    };
    Ok(FieldValue::Text(taken))
}

fn mid(values: &[FieldValue], name: &str) -> Result<FieldValue, ExprError> {
    let Some(text) = text_arg(&values[0]) else {
        return Ok(FieldValue::Null);
    };
    let start = match &values[1] {
        FieldValue::Null => return Ok(FieldValue::Null),
        other => to_i64(other)?,
    };
    if start < 1 {
        return Err(runtime(format!("{name} expects a start of at least 1")));
    }
    let length = match values.get(2) {
        None | Some(FieldValue::Null) => None,
        Some(other) => {
            let length = to_i64(other)?;
            if length < 0 {
                return Err(runtime(format!("{name} expects a non-negative length")));
            }
            Some(length as usize)
        }
    };
    let chars: Vec<char> = text.chars().skip((start - 1) as usize).collect();
    let taken = match length {
        Some(length) => chars[..length.min(chars.len())].iter().collect(),
        None => chars.iter().collect(),
    };
    Ok(FieldValue::Text(taken))
}

fn case_or_trim(canonical: &str, value: &FieldValue) -> Result<FieldValue, ExprError> {
    let Some(text) = text_arg(value) else {
        return Ok(FieldValue::Null);
    };
    Ok(FieldValue::Text(match canonical {
        "ucase" => text.to_uppercase(),
        "lcase" => text.to_lowercase(),
        "trim" => text.trim().to_owned(),
        "ltrim" => text.trim_start().to_owned(),
        _ => text.trim_end().to_owned(),
    }))
}

/// `InStr([start,] text, find)` — case-sensitive, 1-based, 0 on miss.
fn instr(values: &[FieldValue], name: &str) -> Result<FieldValue, ExprError> {
    let (start, text, find) = if values.len() == 3 {
        let start = match &values[0] {
            FieldValue::Null => return Ok(FieldValue::Null),
            other => to_i64(other)?,
        };
        if start < 1 {
            return Err(runtime(format!("{name} expects a start of at least 1")));
        }
        (start, &values[1], &values[2])
    } else {
        (1, &values[0], &values[1])
    };
    let (Some(text), Some(find)) = (text_arg(text), text_arg(find)) else {
        return Ok(FieldValue::Null);
    };
    if find.is_empty() {
        return Ok(FieldValue::Integer(start));
    }
    let suffix: String = text.chars().skip((start - 1) as usize).collect();
    let position = match suffix.find(find.as_str()) {
        Some(byte_offset) => start + suffix[..byte_offset].chars().count() as i64,
        None => 0,
    };
    Ok(FieldValue::Integer(position))
}

fn abs(value: &FieldValue) -> Result<FieldValue, ExprError> {
    Ok(match value {
        FieldValue::Null => FieldValue::Null,
        FieldValue::Integer(i) => {
            FieldValue::Integer(i.checked_abs().ok_or_else(|| runtime("integer overflow"))?)
        }
        FieldValue::Currency(c) => {
            FieldValue::Currency(c.checked_abs().ok_or_else(|| runtime("integer overflow"))?)
        }
        FieldValue::Decimal(d) => {
            FieldValue::Decimal(d.checked_abs().ok_or_else(|| runtime("integer overflow"))?)
        }
        FieldValue::Float(f) | FieldValue::DateTime(f) => FieldValue::Float(f.abs()),
        other => {
            return Err(runtime(format!(
                "{} values have no absolute value",
                other.family()
            )));
        }
    })
}

fn floor_or_trunc(canonical: &str, value: &FieldValue) -> Result<FieldValue, ExprError> {
    Ok(match value {
        FieldValue::Null => FieldValue::Null,
        FieldValue::Integer(i) => FieldValue::Integer(*i),
        FieldValue::Float(f) | FieldValue::DateTime(f) => {
            let rounded = if canonical == "int" {
                f.floor()
            } else {
                f.trunc()
            };
            FieldValue::Integer(
                i64::try_from(rounded as i128)
                    .map_err(|_| runtime("value does not fit the integer range"))?,
            )
        }
        FieldValue::Currency(c) => {
            let value = *c as f64 / 10_000.0;
            let rounded = if canonical == "int" {
                value.floor()
            } else {
                value.trunc()
            };
            FieldValue::Integer(rounded as i64)
        }
        other => {
            return Err(runtime(format!(
                "{} values have no integer part",
                other.family()
            )));
        }
    })
}

fn round(values: &[FieldValue], name: &str) -> Result<FieldValue, ExprError> {
    let digits = match values.get(1) {
        None | Some(FieldValue::Null) => 0,
        Some(other) => {
            let digits = to_i64(other)?;
            if !(0..=15).contains(&digits) {
                return Err(runtime(format!(
                    "{name} expects 0..=15 digits, found {digits}"
                )));
            }
            digits
        }
    };
    Ok(match &values[0] {
        FieldValue::Null => FieldValue::Null,
        FieldValue::Integer(i) => FieldValue::Integer(*i),
        FieldValue::Currency(c) => {
            let value = *c as f64 / 10_000.0;
            FieldValue::Currency((bankers(value, digits) * 10_000.0).round() as i64)
        }
        FieldValue::Float(f) | FieldValue::DateTime(f) => FieldValue::Float(bankers(*f, digits)),
        other => {
            return Err(runtime(format!("{} values cannot round", other.family())));
        }
    })
}

/// VBA `Round`: half to even at the requested digit.
fn bankers(value: f64, digits: i64) -> f64 {
    let factor = 10f64.powi(digits as i32);
    (value * factor).round_ties_even() / factor
}

fn sqr(value: &FieldValue, name: &str) -> Result<FieldValue, ExprError> {
    Ok(match value {
        FieldValue::Null => FieldValue::Null,
        value => {
            let number = match value {
                FieldValue::Integer(i) => *i as f64,
                FieldValue::Float(f) | FieldValue::DateTime(f) => *f,
                FieldValue::Currency(c) => *c as f64 / 10_000.0,
                other => {
                    return Err(runtime(format!(
                        "{name} expects a numeric value, found {}",
                        other.family()
                    )));
                }
            };
            if number < 0.0 {
                return Err(runtime(format!(
                    "{name} expects a non-negative number, found {number}"
                )));
            }
            FieldValue::Float(number.sqrt())
        }
    })
}

#[cfg(test)]
mod tests;

/// Built-in edge-path pins (step 43.1, item T9): conversion families,
/// rounding views and the function-name error prefix.
#[cfg(test)]
mod coverage_tests {
    use super::*;

    fn eval_ok(source: &str, scope: &Scope) -> FieldValue {
        super::super::parse(source)
            .and_then(|expr| expr.evaluate(scope))
            .unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
    }

    /// Conversion and numeric built-ins pin their per-family branches:
    /// numeric truth, date construction, the currency/scalar views of
    /// rounding and square roots (direct calls — the language has no
    /// currency literal) and the argument-failure prefix (step 43.1, T9).
    #[test]
    fn builtins_pin_their_family_branches() {
        let scope = Scope::new();
        assert_eq!(eval_ok("isnumeric(2)", &scope), FieldValue::Boolean(true));
        assert_eq!(
            eval_ok("isnumeric(\"x\")", &scope),
            FieldValue::Boolean(false)
        );
        assert_eq!(
            eval_ok("isnumeric(True)", &scope),
            FieldValue::Boolean(true)
        );
        assert_eq!(eval_ok("dateserial(Null, 1, 5)", &scope), FieldValue::Null);
        assert_eq!(
            eval_ok("instr(1, \"abc\", \"\")", &scope),
            FieldValue::Integer(1)
        );
        let error = super::super::parse("Sqr(1/0)")
            .and_then(|expr| expr.evaluate(&scope))
            .expect_err("division by zero must propagate");
        assert!(
            error.to_string().contains("Sqr: division by zero"),
            "{error}"
        );
        let currency = Expression::Literal(FieldValue::Currency(125_000));
        assert_eq!(
            call(
                "round",
                &[
                    currency.clone(),
                    Expression::Literal(FieldValue::Integer(1))
                ],
                &scope
            ),
            Ok(FieldValue::Currency(125_000))
        );
        assert_eq!(
            call("round", &[currency], &scope),
            Ok(FieldValue::Currency(120_000))
        );
        assert_eq!(
            call(
                "sqr",
                &[Expression::Literal(FieldValue::Currency(40_000))],
                &scope
            ),
            Ok(FieldValue::Float(2.0))
        );
        let error = call(
            "round",
            &[Expression::Literal(FieldValue::Boolean(true))],
            &scope,
        )
        .expect_err("booleans do not round");
        assert!(error.to_string().contains("cannot round"), "{error}");
    }
}
