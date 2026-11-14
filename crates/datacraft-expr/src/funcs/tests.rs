//! Built-in edge-path pins (step 43.1, item T9): conversion families,
//! rounding views and the function-name error prefix.

use super::*;

fn eval_ok(source: &str, scope: &Scope) -> FieldValue {
    super::super::parse(source)
        .and_then(|expr| expr.evaluate(scope))
        .unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
}

#[test]
fn arity_errors_name_the_function() {
    let error = super::super::parse("Left(\"a\")")
        .and_then(|expr| expr.evaluate(&Scope::new()))
        .expect_err("arity must fail");
    assert!(error.to_string().contains("Left expects"));
}

#[test]
fn unknown_functions_fail_at_runtime() {
    let error = super::super::parse("Foo(1)")
        .and_then(|expr| expr.evaluate(&Scope::new()))
        .expect_err("unknown function must fail");
    assert!(error.to_string().contains("unknown function Foo"));
}

#[test]
fn date_serial_normalizes_overflow() {
    assert_eq!(
        eval_ok("DateSerial(2024, 13, 1)", &Scope::new()),
        FieldValue::DateTime(45_658.0)
    );
    assert_eq!(
        eval_ok("DateSerial(2025, 1, 32)", &Scope::new()),
        FieldValue::DateTime(45_689.0)
    );
}

#[test]
fn nz_defaults_and_isnumeric_propagation() {
    assert_eq!(
        eval_ok(
            "Nz([n], \"fallback\")",
            Scope::new().set("n", FieldValue::Null)
        ),
        FieldValue::Text("fallback".into())
    );
    assert_eq!(
        eval_ok("IsNumeric([n])", Scope::new().set("n", FieldValue::Null)),
        FieldValue::Null
    );
}

#[test]
fn round_uses_bankers_halves() {
    assert_eq!(eval_ok("Round(2.5)", &Scope::new()), FieldValue::Float(2.0));
    assert_eq!(eval_ok("Round(3.5)", &Scope::new()), FieldValue::Float(4.0));
}
