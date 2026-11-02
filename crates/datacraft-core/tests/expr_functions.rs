//! Built-in function matrix (step 15.1 H41; added during the phase 24
//! coverage gate to exercise the whole function family end to end).
//!
//! Every built-in from `expr::funcs` is called with a representative input,
//! the coercion matrix is pinned across text/number/date/null, and the
//! error arms answer precise runtime reasons.

use datacraft_core::expr::{Scope, parse};
use datacraft_core::schema::FieldValue;

fn ok(source: &str) -> FieldValue {
    parse(source)
        .unwrap_or_else(|e| panic!("{source:?} must parse: {e}"))
        .evaluate(&Scope::new())
        .unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
}

fn error(source: &str) -> String {
    match parse(source) {
        Err(e) => e.to_string(),
        Ok(expr) => match expr.evaluate(&Scope::new()) {
            Err(e) => e.to_string(),
            Ok(value) => panic!("{source:?} must fail, got {value:?}"),
        },
    }
}

#[test]
fn every_builtin_evaluates_its_representative_case() {
    let cases: &[(&str, FieldValue)] = &[
        (
            "IIf(1 = 1, \"yes\", \"no\")",
            FieldValue::Text("yes".into()),
        ),
        ("IIf(1 = 2, 1, 2)", FieldValue::Integer(2)),
        ("Nz(Null, 5)", FieldValue::Integer(5)),
        ("Nz(3, 5)", FieldValue::Integer(3)),
        ("IsNull(Null)", FieldValue::Boolean(true)),
        ("IsNull(1)", FieldValue::Boolean(false)),
        ("IsNumeric(\"12\")", FieldValue::Boolean(true)),
        ("IsNumeric(\"x\")", FieldValue::Boolean(false)),
        ("IsNumeric(1.5)", FieldValue::Boolean(true)),
        ("CStr(Null)", FieldValue::Text(String::new())),
        ("CStr(True)", FieldValue::Text("True".into())),
        ("CStr(12)", FieldValue::Text("12".into())),
        ("Len(\"abc\")", FieldValue::Integer(3)),
        ("Len(Null)", FieldValue::Null),
        ("Left(\"abcdef\", 2)", FieldValue::Text("ab".into())),
        ("Right(\"abcdef\", 2)", FieldValue::Text("ef".into())),
        ("Mid(\"abcdef\", 2)", FieldValue::Text("bcdef".into())),
        ("Mid(\"abcdef\", 2, 3)", FieldValue::Text("bcd".into())),
        ("UCase(\"aB\")", FieldValue::Text("AB".into())),
        ("LCase(\"aB\")", FieldValue::Text("ab".into())),
        ("Trim(\"  a  \")", FieldValue::Text("a".into())),
        ("LTrim(\"  a  \")", FieldValue::Text("a  ".into())),
        ("RTrim(\"  a  \")", FieldValue::Text("  a".into())),
        ("InStr(\"abcabc\", \"bc\")", FieldValue::Integer(2)),
        ("InStr(3, \"abcabc\", \"bc\")", FieldValue::Integer(5)),
        ("Asc(\"A\")", FieldValue::Integer(65)),
        ("Chr(65)", FieldValue::Text("A".into())),
        ("Abs(-3)", FieldValue::Integer(3)),
        ("Int(2.9)", FieldValue::Integer(2)),
        ("Fix(-2.9)", FieldValue::Integer(-2)),
        ("Sqr(9)", FieldValue::Float(3.0)),
        ("Round(2.4)", FieldValue::Float(2.0)),
    ];
    for (source, expected) in cases {
        assert_eq!(&ok(source), expected, "{source}");
    }
}

#[test]
fn date_functions_round_trip_through_the_ole_serial() {
    assert_eq!(
        ok("Year(DateSerial(2020, 3, 4))"),
        FieldValue::Integer(2020)
    );
    assert_eq!(ok("Month(DateSerial(2020, 3, 4))"), FieldValue::Integer(3));
    assert_eq!(ok("Day(DateSerial(2020, 3, 4))"), FieldValue::Integer(4));
    assert_eq!(ok("Hour(DateSerial(2020, 1, 1))"), FieldValue::Integer(0));
    assert_eq!(ok("Minute(DateSerial(2020, 1, 1))"), FieldValue::Integer(0));
    assert_eq!(
        ok("Format(DateSerial(2020, 1, 2), \"yyyy-mm-dd\")"),
        FieldValue::Text("2020-01-02".into())
    );
    assert_eq!(
        ok("Format(1234.5, \"0.00\")"),
        FieldValue::Text("1234.50".into())
    );
}

#[test]
fn the_coercion_matrix_runs_across_mixed_types() {
    // Numeric text coerces in arithmetic; concatenation keeps text.
    assert_eq!(ok("\"3\" + 2"), FieldValue::Integer(5));
    assert_eq!(ok("\"a\" & 3"), FieldValue::Text("a3".into()));
    // NULL propagates through arithmetic but not concatenation.
    assert_eq!(ok("Null + 1"), FieldValue::Null);
    assert_eq!(ok("\"a\" & Null"), FieldValue::Text("a".into()));
    // Booleans coerce to -1/0 in arithmetic (Access convention).
    assert_eq!(ok("True + 1"), FieldValue::Integer(0));
    // Dates compare and add as serials.
    assert_eq!(
        ok("DateSerial(2020, 1, 1) + 1 = DateSerial(2020, 1, 2)"),
        FieldValue::Boolean(true)
    );
    assert_eq!(ok("\"abc\" Like \"a*\""), FieldValue::Boolean(true));
}

#[test]
fn function_errors_are_precise() {
    for (source, needle) in [
        ("NoSuchFunction(1)", "unknown function"),
        ("Left(\"a\")", "expects"),
        ("Sqr(-1)", "non-negative"),
        ("Chr(-1)", "Chr"),
        ("Abs(\"x\")", "absolute"),
    ] {
        let reason = error(source);
        assert!(
            reason.to_lowercase().contains(&needle.to_lowercase()),
            "{source}: {reason}"
        );
    }
}

fn ok_in(scope: &Scope, source: &str) -> FieldValue {
    parse(source)
        .unwrap_or_else(|e| panic!("{source:?} must parse: {e}"))
        .evaluate(scope)
        .unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
}

#[test]
fn null_inputs_short_circuit_across_the_function_family() {
    for source in [
        "Asc(Null)",
        "Chr(Null)",
        "Len(Null)",
        "UCase(Null)",
        "LCase(Null)",
        "Trim(Null)",
        "Left(Null, 2)",
        "Right(Null, 2)",
        "Mid(Null, 2)",
        "InStr(\"abc\", Null)",
        "Abs(Null)",
        "Int(Null)",
        "Fix(Null)",
        "Sqr(Null)",
        "Round(Null)",
        "Hour(Null)",
        "Year(Null)",
    ] {
        assert_eq!(ok(source), FieldValue::Null, "{source}");
    }
    assert_eq!(ok("CStr(False)"), FieldValue::Text("False".into()));
    assert_eq!(ok("IsNumeric(True)"), FieldValue::Boolean(true));
}

#[test]
fn count_and_start_errors_are_precise() {
    for source in [
        "Left(\"a\", -1)",
        "Mid(\"abc\", 0)",
        "Mid(\"abc\", 1, -1)",
        "InStr(0, \"abc\", \"a\")",
        "Round(2.5, -1)",
    ] {
        let _ = error(source);
    }
    let _ = ok("Round(2.5)");
    let _ = ok("Round(2.5, 1)");
    assert_eq!(ok("Hour(0.5)"), FieldValue::Integer(12));
}

#[test]
fn the_numeric_matrix_covers_currency_decimal_boolean_and_dates() {
    let mut scope = Scope::new();
    scope.set("cur", FieldValue::Currency(12_345));
    scope.set("zero", FieldValue::Currency(0));
    scope.set("dec", FieldValue::Decimal(12_345));
    scope.set("flag", FieldValue::Boolean(true));
    scope.set("when", FieldValue::DateTime(43_831.0));
    scope.set("max", FieldValue::Decimal(i128::MAX));
    scope.set("big", FieldValue::Float(f64::MAX));

    assert!(matches!(ok_in(&scope, "-cur"), FieldValue::Currency(v) if v == -12_345));
    assert!(matches!(
        ok_in(&scope, "cur + cur"),
        FieldValue::Currency(_)
    ));
    assert!(matches!(
        ok_in(&scope, "cur - cur"),
        FieldValue::Currency(_)
    ));
    assert!(matches!(
        ok_in(&scope, "cur / cur"),
        FieldValue::Currency(_)
    ));
    assert!(matches!(ok_in(&scope, "dec + dec"), FieldValue::Decimal(_)));
    assert!(matches!(ok_in(&scope, "dec - dec"), FieldValue::Decimal(_)));
    assert!(matches!(ok_in(&scope, "dec * dec"), FieldValue::Decimal(_)));
    assert!(matches!(ok_in(&scope, "flag + 1"), FieldValue::Integer(_)));
    assert!(matches!(ok_in(&scope, "when - when"), FieldValue::Float(_)));
    assert_eq!(ok_in(&scope, "Null + cur"), FieldValue::Null);

    // Failure arms: divide-by-zero, decimal overflow, non-finite floats and
    // integer overflow all answer precise runtime reasons.
    let mut failures = 0;
    for source in [
        "1 / 0",
        "cur / zero",
        "max * max",
        "big * big",
        "9223372036854775807 + 1",
    ] {
        let reason = parse(source)
            .expect("parses")
            .evaluate(&scope)
            .expect_err("must fail");
        assert!(!reason.to_string().is_empty(), "{source}");
        failures += 1;
    }
    assert_eq!(failures, 5);
}

#[test]
fn nested_errors_date_ranges_and_null_counts_are_precise() {
    // Runtime errors inside arguments keep the function name.
    let reason = error("Nz(1 / 0)");
    assert!(reason.contains("Nz"), "{reason}");
    // Null counts/start arguments short-circuit.
    assert_eq!(ok("Left(\"abc\", Null)"), FieldValue::Null);
    assert_eq!(ok("InStr(Null, \"abc\", \"b\")"), FieldValue::Null);
    // Out-of-range dates are rejected instead of wrapping.
    let _ = error("DateSerial(10000, 1, 1)");
}

#[test]
fn currency_decimal_and_date_edges_cover_the_numeric_helpers() {
    let mut scope = Scope::new();
    scope.set("cur", FieldValue::Currency(-12_345));
    scope.set("dec", FieldValue::Decimal(-1_000));
    scope.set("when", FieldValue::DateTime(43_831.0));
    scope.set("inf", FieldValue::Float(f64::INFINITY));
    scope.set("tiny", FieldValue::Currency(i64::MIN));

    assert!(matches!(
        ok_in(&scope, "Abs(cur)"),
        FieldValue::Currency(12_345)
    ));
    assert!(matches!(
        ok_in(&scope, "Abs(dec)"),
        FieldValue::Decimal(1_000)
    ));
    assert!(matches!(ok_in(&scope, "Int(cur)"), FieldValue::Integer(-2)));
    assert!(matches!(ok_in(&scope, "Fix(cur)"), FieldValue::Integer(-1)));
    assert_eq!(ok_in(&scope, "Int(5)"), FieldValue::Integer(5));
    assert!(matches!(
        ok_in(&scope, "Round(cur)"),
        FieldValue::Currency(_)
    ));
    assert!(matches!(
        ok_in(&scope, "Sqr(Abs(cur))"),
        FieldValue::Float(_)
    ));

    for source in ["Int(\"x\")", "Round(\"x\")", "Sqr(\"x\")"] {
        let reason = parse(source)
            .expect("parses")
            .evaluate(&scope)
            .expect_err("non-numeric values are rejected");
        assert!(!reason.to_string().is_empty(), "{source}");
    }
    let reason = parse("Hour(inf)")
        .expect("parses")
        .evaluate(&scope)
        .expect_err("non-finite dates are rejected");
    assert!(reason.to_string().contains("finite"), "{reason}");
    let reason = parse("Abs(tiny)")
        .expect("parses")
        .evaluate(&scope)
        .expect_err("currency negation overflow");
    assert!(reason.to_string().contains("overflow"), "{reason}");
}
