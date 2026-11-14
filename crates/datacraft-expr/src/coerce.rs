//! Value coercion for the expression service (step 15.1, item H41).
//!
//! Every coercion the evaluator and the built-ins share: the numeric
//! tower (Boolean → Integer → Currency → Float, Decimal as raw scaled
//! integer, text that parses), condition coercion (`CBool` rules),
//! display coercion (`Null` as empty string) and family-safe ordering.

use super::ExprError;
use datacraft_types::FieldValue;
use datacraft_types::compare;

/// Numeric coercion tower: Boolean as VBA (-1/0), text that parses, dates
/// as float OLE days, decimals as raw scaled integers.
pub(crate) fn to_number(value: &FieldValue) -> Result<Num, ExprError> {
    Ok(match value {
        FieldValue::Boolean(flag) => Num::Int(if *flag { -1 } else { 0 }),
        FieldValue::Integer(i) => Num::Int(*i),
        FieldValue::Currency(c) => Num::Currency(*c),
        FieldValue::Float(f) | FieldValue::DateTime(f) => Num::Float(*f),
        FieldValue::Decimal(d) => Num::Decimal(*d),
        FieldValue::Text(text) => parse_num_text(text)?,
        other => {
            return Err(runtime(format!(
                "{} values do not participate in arithmetic",
                other.family()
            )));
        }
    })
}

/// A coerced numeric operand (`Num::Float` covers `DateTime` as OLE days).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Num {
    /// Integral operand.
    Int(i64),
    /// Currency in scaled integer form.
    Currency(i64),
    /// Floating-point operand (dates included).
    Float(f64),
    /// Decimal in raw scaled-integer form.
    Decimal(i128),
}

impl From<Num> for f64 {
    fn from(num: Num) -> Self {
        match num {
            Num::Int(i) => i as f64,
            Num::Currency(c) => c as f64 / 10_000.0,
            Num::Float(f) => f,
            Num::Decimal(d) => d as f64,
        }
    }
}

impl Num {
    /// Condition view: is the value non-zero (VBA `CBool`).
    pub(crate) fn non_zero(self) -> bool {
        match self {
            Self::Int(i) => i != 0,
            Self::Currency(c) => c != 0,
            Self::Float(f) => f != 0.0,
            Self::Decimal(d) => d != 0,
        }
    }
}

/// Parses numeric text (`"5"` → 6 in `"5" + 1`); anything else is a
/// precise runtime error.
pub(crate) fn parse_num_text(text: &str) -> Result<Num, ExprError> {
    let trimmed = text.trim();
    if let Ok(i) = trimmed.parse::<i64>() {
        return Ok(Num::Int(i));
    }
    if let Ok(f) = trimmed.parse::<f64>()
        && f.is_finite()
    {
        return Ok(Num::Float(f));
    }
    Err(runtime(format!("{text:?} is not numeric")))
}

/// Integer view of a value for `\`/`Mod`/index-like arguments.
pub(crate) fn to_i64(value: &FieldValue) -> Result<i64, ExprError> {
    let num = to_number(value)?;
    Ok(match num {
        Num::Int(i) => i,
        other => vba_round(f64::from(other)),
    })
}

/// VBA banker's rounding (`CLng`): halves go to the even neighbor.
pub(crate) fn vba_round(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// Unary negation with Null propagation and per-family overflow checks.
pub(crate) fn negate(value: &FieldValue) -> Result<FieldValue, ExprError> {
    let overflow = || runtime("integer overflow");
    Ok(match value {
        FieldValue::Null => FieldValue::Null,
        FieldValue::Boolean(flag) => FieldValue::Integer(if *flag { 1 } else { 0 }),
        FieldValue::Integer(i) => FieldValue::Integer(i.checked_neg().ok_or_else(overflow)?),
        FieldValue::Currency(c) => FieldValue::Currency(c.checked_neg().ok_or_else(overflow)?),
        FieldValue::Decimal(d) => FieldValue::Decimal(d.checked_neg().ok_or_else(overflow)?),
        FieldValue::Float(f) | FieldValue::DateTime(f) => FieldValue::Float(-f),
        FieldValue::Text(text) => match parse_num_text(text)? {
            Num::Int(i) => FieldValue::Integer(i.checked_neg().ok_or_else(overflow)?),
            Num::Currency(c) => FieldValue::Currency(c.checked_neg().ok_or_else(overflow)?),
            Num::Decimal(d) => FieldValue::Decimal(d.checked_neg().ok_or_else(overflow)?),
            Num::Float(f) => FieldValue::Float(-f),
        },
        other => {
            return Err(runtime(format!(
                "{} values cannot be negated",
                other.family()
            )));
        }
    })
}

/// `Option<bool>` view: `None` is the third (Null) truth state. Booleans
/// stay themselves; numbers are non-zero; text follows `CBool`
/// (`"true"`/`"yes"`/`"no"`/`"false"`, numeric text, else error).
pub(crate) fn bool_state(value: &FieldValue) -> Result<Option<bool>, ExprError> {
    Ok(match value {
        FieldValue::Null => None,
        FieldValue::Boolean(flag) => Some(*flag),
        FieldValue::Integer(i) => Some(*i != 0),
        FieldValue::Currency(c) => Some(*c != 0),
        FieldValue::Decimal(d) => Some(*d != 0),
        FieldValue::Float(f) | FieldValue::DateTime(f) => {
            if f.is_finite() {
                Some(*f != 0.0)
            } else {
                return Err(runtime("boolean coercion needs a finite number"));
            }
        }
        FieldValue::Text(text) => match text.trim().to_lowercase().as_str() {
            "true" | "yes" => Some(true),
            "false" | "no" => Some(false),
            other => Some(parse_num_text(other)?.non_zero()),
        },
        other => {
            return Err(runtime(format!(
                "{} values cannot act as conditions",
                other.family()
            )));
        }
    })
}

/// Renders a value for `&`/text `+`/`CStr` fallbacks: Null is the empty
/// string, everything else goes through the shared presentation layer.
pub(crate) fn display(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => String::new(),
        FieldValue::Text(text) => text.clone(),
        other => datacraft_types::display_value(other),
    }
}

/// Family-safe ordering: same families through the engine's comparison,
/// cross-family numerics through float, numeric text coerced.
pub(crate) fn ord_cmp(a: &FieldValue, b: &FieldValue) -> Result<std::cmp::Ordering, ExprError> {
    if let Some(ordering) = compare(a, b) {
        return Ok(ordering);
    }
    let (x, y) = match (cross_f64(a), cross_f64(b)) {
        (Some(x), Some(y)) => (x, y),
        _ if matches!(a, FieldValue::Text(_)) && cross_f64(b).is_some() => {
            (numeric_f64(a)?, cross_f64(b).expect("checked above"))
        }
        _ if matches!(b, FieldValue::Text(_)) && cross_f64(a).is_some() => {
            (cross_f64(a).expect("checked above"), numeric_f64(b)?)
        }
        _ => {
            return Err(runtime(format!(
                "cannot compare {} and {}",
                a.family(),
                b.family()
            )));
        }
    };
    x.partial_cmp(&y)
        .ok_or_else(|| runtime("comparison needs finite numbers"))
}

fn numeric_f64(value: &FieldValue) -> Result<f64, ExprError> {
    match value {
        FieldValue::Text(text) => Ok(f64::from(parse_num_text(text)?)),
        other => Ok(f64::from(to_number(other)?)),
    }
}

/// The float view used for cross-family comparisons (decimals excluded —
/// their scale lives in the field, not the value).
fn cross_f64(value: &FieldValue) -> Option<f64> {
    match value {
        FieldValue::Boolean(flag) => Some(if *flag { -1.0 } else { 0.0 }),
        FieldValue::Integer(i) => Some(*i as f64),
        FieldValue::Currency(c) => Some(*c as f64 / 10_000.0),
        FieldValue::Float(f) | FieldValue::DateTime(f) => Some(*f),
        _ => None,
    }
}

pub(crate) fn runtime(detail: impl Into<String>) -> ExprError {
    ExprError::Runtime {
        detail: detail.into(),
    }
}

/// The arithmetic operator family of `+ - * /`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Arithmetic {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
}

/// Arithmetic with the documented Jet semantics: Null propagates, the
/// result family follows the operands (integer stays integer, currency
/// pairs stay currency, decimals stay decimal, everything else widens to
/// float), `/` always widens except within currency.
pub(crate) fn arith(
    op: Arithmetic,
    a: &FieldValue,
    b: &FieldValue,
) -> Result<FieldValue, ExprError> {
    if matches!(a, FieldValue::Null) || matches!(b, FieldValue::Null) {
        return Ok(FieldValue::Null);
    }
    let (x, y) = (to_number(a)?, to_number(b)?);
    match (op, x, y) {
        (Arithmetic::Divide, Num::Int(x), Num::Int(y)) => {
            if y == 0 {
                return Err(runtime("division by zero"));
            }
            Ok(FieldValue::Float(x as f64 / y as f64))
        }
        (Arithmetic::Divide, x, y) if is_currency_pair(x, y) => {
            let (xs, ys) = scaled_pair(x, y)?;
            if ys == 0 {
                return Err(runtime("division by zero"));
            }
            Ok(FieldValue::Currency(
                (xs as f64 / ys as f64 * 10_000.0).round() as i64,
            ))
        }
        (Arithmetic::Divide, x, y) => {
            let divisor = f64::from(y);
            if divisor == 0.0 {
                return Err(runtime("division by zero"));
            }
            finite_float(f64::from(x) / divisor)
        }
        (op, Num::Int(x), Num::Int(y)) => int_op(op, x, y).map(FieldValue::Integer),
        (op, x, y) if is_currency_pair(x, y) => {
            let (xs, ys) = scaled_pair(x, y)?;
            currency_op(op, xs, ys).map(FieldValue::Currency)
        }
        (op, x, y) if is_decimal_pair(x, y) => {
            dec_op(op, raw_scaled(x), raw_scaled(y)).map(FieldValue::Decimal)
        }
        (op, x, y) => {
            let result = match op {
                Arithmetic::Add => f64::from(x) + f64::from(y),
                Arithmetic::Subtract => f64::from(x) - f64::from(y),
                Arithmetic::Multiply => f64::from(x) * f64::from(y),
                Arithmetic::Divide => unreachable!("divided above"),
            };
            finite_float(result)
        }
    }
}

fn finite_float(result: f64) -> Result<FieldValue, ExprError> {
    if result.is_finite() {
        Ok(FieldValue::Float(result))
    } else {
        Err(runtime("arithmetic result is not finite"))
    }
}

fn is_currency_pair(x: Num, y: Num) -> bool {
    matches!(
        (x, y),
        (Num::Currency(_), Num::Currency(_))
            | (Num::Currency(_), Num::Int(_))
            | (Num::Int(_), Num::Currency(_))
    )
}

fn is_decimal_pair(x: Num, y: Num) -> bool {
    matches!(
        (x, y),
        (Num::Decimal(_), Num::Decimal(_))
            | (Num::Decimal(_), Num::Int(_))
            | (Num::Int(_), Num::Decimal(_))
    )
}

/// Scaled integer view of a currency pair (integers enter the scale).
fn scaled_pair(x: Num, y: Num) -> Result<(i64, i64), ExprError> {
    let scale = |num: Num| -> Result<i64, ExprError> {
        match num {
            Num::Currency(c) => Ok(c),
            Num::Int(i) => i
                .checked_mul(10_000)
                .ok_or_else(|| runtime("integer overflow")),
            other => Err(runtime(format!(
                "{} cannot enter a currency operand",
                match other {
                    Num::Float(_) => "float",
                    Num::Decimal(_) => "decimal",
                    _ => unreachable!("only float/decimal fall through"),
                }
            ))),
        }
    };
    Ok((scale(x)?, scale(y)?))
}

fn raw_scaled(num: Num) -> i128 {
    match num {
        Num::Decimal(d) => d,
        Num::Int(i) => i128::from(i),
        Num::Currency(c) => i128::from(c),
        Num::Float(f) => f as i128,
    }
}

/// Currency arithmetic on scaled integers; multiply divides the double
/// scale back down (`12.5 × 2` stays `25`).
fn currency_op(op: Arithmetic, xs: i64, ys: i64) -> Result<i64, ExprError> {
    match op {
        Arithmetic::Add => xs.checked_add(ys),
        Arithmetic::Subtract => xs.checked_sub(ys),
        Arithmetic::Multiply => Some(((xs as f64) * (ys as f64) / 10_000.0).round() as i64),
        Arithmetic::Divide => unreachable!("divided above"),
    }
    .ok_or_else(|| runtime("integer overflow"))
}

fn int_op(op: Arithmetic, x: i64, y: i64) -> Result<i64, ExprError> {
    match op {
        Arithmetic::Add => x.checked_add(y),
        Arithmetic::Subtract => x.checked_sub(y),
        Arithmetic::Multiply => x.checked_mul(y),
        Arithmetic::Divide => unreachable!("divided above"),
    }
    .ok_or_else(|| runtime("integer overflow"))
}

fn dec_op(op: Arithmetic, x: i128, y: i128) -> Result<i128, ExprError> {
    match op {
        Arithmetic::Add => x.checked_add(y),
        Arithmetic::Subtract => x.checked_sub(y),
        Arithmetic::Multiply => x.checked_mul(y),
        Arithmetic::Divide => {
            if y == 0 {
                return Err(runtime("division by zero"));
            }
            Some((x as f64 / y as f64).round() as i128)
        }
    }
    .ok_or_else(|| runtime("integer overflow"))
}

/// `\` and `Mod`: operands round to integers first (VBA banker's
/// rounding), results truncate toward zero and take the dividend's sign.
pub(crate) fn integer_arith(
    modulo: bool,
    a: &FieldValue,
    b: &FieldValue,
) -> Result<FieldValue, ExprError> {
    if matches!(a, FieldValue::Null) || matches!(b, FieldValue::Null) {
        return Ok(FieldValue::Null);
    }
    let x = to_i64(a)?;
    let y = to_i64(b)?;
    if y == 0 {
        return Err(runtime("division by zero"));
    }
    Ok(FieldValue::Integer(if modulo { x % y } else { x / y }))
}

#[cfg(test)]
mod tests;
