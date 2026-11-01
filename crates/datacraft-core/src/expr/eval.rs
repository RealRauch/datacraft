//! Expression evaluation (step 15.1, item H41).
//!
//! Total reduction over the AST with the documented Jet semantics: NULL
//! propagation, three-valued logic and family-safe comparison. The value
//! coercions live in [`super::coerce`], functions dispatch through
//! [`super::funcs`], `Like` through [`super::like`].

use super::coerce::{
    Arithmetic, arith, bool_state, display, integer_arith, negate, ord_cmp, runtime, to_number,
};
use super::funcs;
use super::like;
use super::{BinaryOp, CompareOp, ExprError, Expression, Scope, UnaryOp};
use crate::schema::FieldValue;

pub(crate) fn evaluate(expression: &Expression, scope: &Scope) -> Result<FieldValue, ExprError> {
    match expression {
        Expression::Literal(value) => Ok(value.clone()),
        Expression::Field { table, name } => match scope.resolve(table.as_deref(), name) {
            Some(value) => Ok(value.clone()),
            None => Err(runtime(format!(
                "field {} not found",
                display_field(table, name)
            ))),
        },
        Expression::Unary(op, operand) => {
            let value = evaluate(operand, scope)?;
            match op {
                UnaryOp::Negate => negate(&value),
                UnaryOp::Not => Ok(bool_to_value(not3(bool_state(&value)?))),
            }
        }
        Expression::Binary(op, left, right) => {
            let a = evaluate(left, scope)?;
            let b = evaluate(right, scope)?;
            binary(*op, &a, &b)
        }
        Expression::Compare(op, left, right) => {
            let a = evaluate(left, scope)?;
            let b = evaluate(right, scope)?;
            compare_values(*op, &a, &b)
        }
        Expression::Between(operand, low, high) => {
            let value = evaluate(operand, scope)?;
            let low = evaluate(low, scope)?;
            let high = evaluate(high, scope)?;
            let ge = relation_state(CompareOp::GreaterEqual, &value, &low)?;
            let le = relation_state(CompareOp::LessEqual, &value, &high)?;
            Ok(bool_to_value(and3(ge, le)))
        }
        Expression::InList(operand, list) => {
            let value = evaluate(operand, scope)?;
            if matches!(value, FieldValue::Null) {
                return Ok(FieldValue::Null);
            }
            let mut saw_null = false;
            for element in list {
                let candidate = evaluate(element, scope)?;
                if relation_state(CompareOp::Equal, &value, &candidate)? == Some(true) {
                    return Ok(FieldValue::Boolean(true));
                }
                saw_null |= matches!(candidate, FieldValue::Null);
            }
            Ok(if saw_null {
                FieldValue::Null
            } else {
                FieldValue::Boolean(false)
            })
        }
        Expression::IsNull { operand, negated } => {
            let is_null = matches!(evaluate(operand, scope)?, FieldValue::Null);
            Ok(FieldValue::Boolean(is_null != *negated))
        }
        Expression::Call { name, args } => funcs::call(name, args, scope),
    }
}

fn binary(op: BinaryOp, a: &FieldValue, b: &FieldValue) -> Result<FieldValue, ExprError> {
    match op {
        BinaryOp::Concat => Ok(FieldValue::Text(format!("{}{}", display(a), display(b)))),
        BinaryOp::Add if matches!(a, FieldValue::Text(_)) && matches!(b, FieldValue::Text(_)) => {
            Ok(FieldValue::Text(format!("{}{}", display(a), display(b))))
        }
        BinaryOp::Add => arith(Arithmetic::Add, a, b),
        BinaryOp::Subtract => arith(Arithmetic::Subtract, a, b),
        BinaryOp::Multiply => arith(Arithmetic::Multiply, a, b),
        BinaryOp::Divide => arith(Arithmetic::Divide, a, b),
        BinaryOp::IntegerDivide => integer_arith(false, a, b),
        BinaryOp::Modulo => integer_arith(true, a, b),
        BinaryOp::Power => {
            if matches!(a, FieldValue::Null) || matches!(b, FieldValue::Null) {
                return Ok(FieldValue::Null);
            }
            let result = f64::from(to_number(a)?).powf(f64::from(to_number(b)?));
            if result.is_finite() {
                Ok(FieldValue::Float(result))
            } else {
                Err(runtime("power result is not finite"))
            }
        }
        BinaryOp::And => Ok(bool_to_value(and3(bool_state(a)?, bool_state(b)?))),
        BinaryOp::Or => Ok(bool_to_value(or3(bool_state(a)?, bool_state(b)?))),
    }
}

fn and3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

fn or3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

fn not3(a: Option<bool>) -> Option<bool> {
    a.map(|flag| !flag)
}

fn bool_to_value(state: Option<bool>) -> FieldValue {
    match state {
        Some(flag) => FieldValue::Boolean(flag),
        None => FieldValue::Null,
    }
}

fn display_field(table: &Option<String>, name: &str) -> String {
    match table {
        Some(table) => format!("{table}.{name}"),
        None => name.to_owned(),
    }
}

fn compare_values(op: CompareOp, a: &FieldValue, b: &FieldValue) -> Result<FieldValue, ExprError> {
    if op == CompareOp::Like {
        return like_values(a, b);
    }
    Ok(bool_to_value(relation_state(op, a, b)?))
}

/// One comparison leg (also of `Between`/`In`): `Some` truth or `None`
/// for Null operands (Jet three-valued propagation).
fn relation_state(
    op: CompareOp,
    a: &FieldValue,
    b: &FieldValue,
) -> Result<Option<bool>, ExprError> {
    if matches!(a, FieldValue::Null) || matches!(b, FieldValue::Null) {
        return Ok(None);
    }
    let ordering = ord_cmp(a, b)?;
    Ok(Some(match op {
        CompareOp::Equal => ordering == std::cmp::Ordering::Equal,
        CompareOp::NotEqual => ordering != std::cmp::Ordering::Equal,
        CompareOp::Less => ordering == std::cmp::Ordering::Less,
        CompareOp::LessEqual => ordering != std::cmp::Ordering::Greater,
        CompareOp::Greater => ordering == std::cmp::Ordering::Greater,
        CompareOp::GreaterEqual => ordering != std::cmp::Ordering::Less,
        CompareOp::Like => unreachable!("handled in compare_values"),
    }))
}

fn like_values(a: &FieldValue, b: &FieldValue) -> Result<FieldValue, ExprError> {
    if matches!(a, FieldValue::Null) || matches!(b, FieldValue::Null) {
        return Ok(FieldValue::Null);
    }
    let pattern = match b {
        FieldValue::Text(text) => text.clone(),
        other => {
            return Err(runtime(format!(
                "Like patterns must be text, found {}",
                other.family()
            )));
        }
    };
    Ok(FieldValue::Boolean(like::matches(&display(a), &pattern)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_ok(source: &str, scope: &Scope) -> FieldValue {
        super::super::parse(source)
            .and_then(|expr| expr.evaluate(scope))
            .unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
    }

    #[test]
    fn overflow_is_a_precise_runtime_error() {
        let error = super::super::parse("9223372036854775807 + 1")
            .and_then(|expr| expr.evaluate(&Scope::new()))
            .expect_err("overflow must fail");
        assert!(error.to_string().contains("overflow"));
    }

    #[test]
    fn between_combines_three_valued_legs() {
        let mut scope = Scope::new();
        scope.set("n", FieldValue::Null);
        assert_eq!(eval_ok("[n] Between 1 And 3", &scope), FieldValue::Null);
        assert_eq!(eval_ok("2 Between 1 And [n]", &scope), FieldValue::Null);
    }

    #[test]
    fn cross_family_comparison_uses_float_view() {
        let mut scope = Scope::new();
        scope.set("c", FieldValue::Currency(15_000));
        assert_eq!(eval_ok("[c] > 1.2", &scope), FieldValue::Boolean(true));
        scope.set("d", FieldValue::DateTime(2.0));
        assert_eq!(eval_ok("[d] = 2", &scope), FieldValue::Boolean(true));
    }

    #[test]
    fn non_condition_values_are_refused() {
        let error = super::super::parse("[b] And True")
            .and_then(|expr| expr.evaluate(Scope::new().set("b", FieldValue::Binary(vec![1]))))
            .expect_err("binary cannot act as a condition");
        assert!(error.to_string().contains("conditions"));
    }

    #[test]
    fn in_list_follows_three_valued_or_folding() {
        let mut scope = Scope::new();
        scope.set("n", FieldValue::Null);
        assert_eq!(eval_ok("[n] In (1, 2)", &scope), FieldValue::Null);
        // A match wins over a Null leg (True Or Null = True).
        assert_eq!(eval_ok("2 In (2, [n])", &scope), FieldValue::Boolean(true));
        // No match with a Null leg: False Or Null = Null.
        assert_eq!(eval_ok("3 In (1, [n])", &scope), FieldValue::Null);
    }

    #[test]
    fn is_null_never_fails_on_its_operand() {
        let mut scope = Scope::new();
        scope.set("x", FieldValue::Null);
        assert_eq!(eval_ok("[x] Is Null", &scope), FieldValue::Boolean(true));
        assert_eq!(
            eval_ok("[x] Is Not Null", &scope),
            FieldValue::Boolean(false)
        );
        assert_eq!(eval_ok("1 Is Not Null", &scope), FieldValue::Boolean(true));
    }
}
