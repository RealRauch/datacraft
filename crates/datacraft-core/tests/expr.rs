//! Expression service contract (step 15.1, item H41).
//!
//! The evaluator parses and evaluates Access-like expressions: literals,
//! field references, arithmetic and concatenation, comparison and
//! three-valued logic with Jet NULL semantics, the IIf/Nz/Format family,
//! date and string functions, and `Like` with Access wildcards. This
//! suite is the reference contract for the adapter step (15.2) and the
//! later expression rings.

use datacraft_core::expr::{ExprError, Scope, parse};
use datacraft_core::schema::FieldValue;

fn eval(source: &str) -> Result<FieldValue, ExprError> {
    parse(source)?.evaluate(&Scope::new())
}

fn eval_with(source: &str, scope: &Scope) -> Result<FieldValue, ExprError> {
    parse(source)?.evaluate(scope)
}

fn ok(source: &str) -> FieldValue {
    eval(source).unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
}

fn is_runtime_error(source: &str, needle: &str) {
    let error = eval(source).expect_err("source must fail at runtime");
    match error {
        ExprError::Runtime { detail } => {
            assert!(detail.contains(needle), "{source:?}: {detail:?} lacks {needle:?}")
        }
        other => panic!("{source:?}: expected runtime error, got {other:?}"),
    }
}

fn is_parse_error(source: &str) {
    match eval(source) {
        Err(ExprError::Parse { .. }) => {}
        other => panic!("{source:?}: expected parse error, got {other:?}"),
    }
}

#[test]
fn literals_parse_and_evaluate() {
    assert_eq!(ok("42"), FieldValue::Integer(42));
    assert_eq!(ok("-7"), FieldValue::Integer(-7));
    assert_eq!(ok("2.5"), FieldValue::Float(2.5));
    assert_eq!(ok("\"Ada\""), FieldValue::Text("Ada".into()));
    assert_eq!(ok("'Ada'"), FieldValue::Text("Ada".into()));
    assert_eq!(ok("\"say \"\"hi\"\"\""), FieldValue::Text("say \"hi\"".into()));
    assert_eq!(ok("True"), FieldValue::Boolean(true));
    assert_eq!(ok("False"), FieldValue::Boolean(false));
    assert_eq!(ok("Null"), FieldValue::Null);
    // 2025-01-05 in OLE Automation days (1970-01-01 is 25569).
    assert_eq!(ok("#2025-01-05#"), FieldValue::DateTime(45_662.0));
    assert_eq!(ok("#2025-01-05 13:45#"), FieldValue::DateTime(45_662.0 + 825.0 / 1440.0));
}

#[test]
fn precedence_and_associativity_follow_the_documented_lattice() {
    assert_eq!(ok("2+3*4"), FieldValue::Integer(14));
    assert_eq!(ok("(2+3)*4"), FieldValue::Integer(20));
    assert_eq!(ok("2^3^2"), FieldValue::Float(512.0)); // right-associative
    assert_eq!(ok("-2^2"), FieldValue::Float(-4.0)); // ^ binds tighter than unary minus
    assert_eq!(ok("7\\2"), FieldValue::Integer(3));
    assert_eq!(ok("7 Mod 2"), FieldValue::Integer(1));
    assert_eq!(ok("\"a\" & 1+2"), FieldValue::Text("a3".into())); // + before &
    assert_eq!(ok("2 < 3 And 4 < 5"), FieldValue::Boolean(true));
    assert_eq!(ok("Not 2 < 3"), FieldValue::Boolean(false));
}

#[test]
fn field_references_resolve_case_insensitively() {
    let mut scope = Scope::new();
    scope.set("Qty", FieldValue::Integer(3));
    scope.set("UnitPrice", FieldValue::Currency(125_000));
    scope.set_field("Order Details", "Qty", FieldValue::Integer(4));
    assert_eq!(ok_with("[Qty]", &scope), FieldValue::Integer(3));
    assert_eq!(ok_with("[qty]", &scope), FieldValue::Integer(3));
    assert_eq!(
        ok_with("[UnitPrice]*2", &scope),
        FieldValue::Currency(250_000)
    );
    assert_eq!(ok_with("[Order Details].[Qty] * 2", &scope), FieldValue::Integer(8));
    is_runtime_error("Ghost + 1", "Ghost");
    is_runtime_error("[Nope].[Qty] * 2", "Nope");
}

fn ok_with(source: &str, scope: &Scope) -> FieldValue {
    eval_with(source, scope).unwrap_or_else(|e| panic!("{source:?} must evaluate: {e}"))
}

#[test]
fn expressions_drive_a_calculated_row() {
    let mut scope = Scope::new();
    scope.set("Qty", FieldValue::Integer(3));
    scope.set("UnitPrice", FieldValue::Currency(125_000));
    assert_eq!(
        ok_with("[Qty] * [UnitPrice]", &scope),
        FieldValue::Currency(375_000)
    );
    assert_eq!(
        ok_with("IIf([Qty] Is Null, 0, [Qty] * [UnitPrice])", &scope),
        FieldValue::Currency(375_000)
    );
}

#[test]
fn arithmetic_coerces_across_the_numeric_tower() {
    assert_eq!(ok("1 + 2.5"), FieldValue::Float(3.5));
    assert_eq!(ok("2 * 3"), FieldValue::Integer(6));
    assert_eq!(ok("7 / 2"), FieldValue::Float(3.5)); // / always widens
    let mut scope = Scope::new();
    scope.set("c", FieldValue::Currency(10_000));
    assert_eq!(ok_with("[c] + 1", &scope), FieldValue::Currency(20_000));
    assert_eq!(ok("True + 1"), FieldValue::Integer(0)); // VBA True = -1
    assert_eq!(ok("\"5\" + 1"), FieldValue::Integer(6)); // numeric text coerces
    assert_eq!(ok("-7\\2"), FieldValue::Integer(-3)); // \ truncates toward zero
    assert_eq!(ok("7.4 Mod 2"), FieldValue::Integer(1)); // operands round first
    is_runtime_error("\"a\" + 1", "a");
    is_runtime_error("1/0", "zero");
    is_runtime_error("7\\0", "zero");
    is_runtime_error("1 Mod 0", "zero");
}

#[test]
fn concatenation_and_null_semantics() {
    assert_eq!(ok("\"a\" & Null"), FieldValue::Text("a".into()));
    assert_eq!(ok("Null & \"b\""), FieldValue::Text("b".into()));
    assert_eq!(ok("Null & Null"), FieldValue::Text(String::new()));
    assert_eq!(ok("\"a\" + Null"), FieldValue::Null);
    assert_eq!(ok("Null + Null"), FieldValue::Null);
    assert_eq!(ok("\"5\" + \"5\""), FieldValue::Text("55".into()));
    assert_eq!(ok("1 + Null"), FieldValue::Null);
    assert_eq!(ok("-Null"), FieldValue::Null);
    let mut scope = Scope::new();
    scope.set("n", FieldValue::Null);
    assert_eq!(ok_with("[n] & \"!\"", &scope), FieldValue::Text("!".into()));
}

#[test]
fn comparisons_are_three_valued() {
    assert_eq!(ok("1 = 1.0"), FieldValue::Boolean(true));
    assert_eq!(ok("1 < 2.5"), FieldValue::Boolean(true));
    assert_eq!(ok("\"a\" < \"b\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"A\" = \"a\""), FieldValue::Boolean(false)); // engine text order
    assert_eq!(ok("Null = Null"), FieldValue::Null);
    assert_eq!(ok("Null <> 1"), FieldValue::Null);
    assert_eq!(ok("1 > Null"), FieldValue::Null);
}

#[test]
fn logic_propagates_null_three_valued() {
    assert_eq!(ok("Not Null"), FieldValue::Null);
    assert_eq!(ok("True And Null"), FieldValue::Null);
    assert_eq!(ok("False And Null"), FieldValue::Boolean(false));
    assert_eq!(ok("True Or Null"), FieldValue::Boolean(true));
    assert_eq!(ok("False Or Null"), FieldValue::Null);
    assert_eq!(ok("Not True"), FieldValue::Boolean(false));
    assert_eq!(ok("1 And 2"), FieldValue::Boolean(true)); // nonzero is true
}

#[test]
fn like_understands_access_wildcards() {
    assert_eq!(ok("\"K-42\" Like \"K-*\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"k-42\" Like \"K-*\""), FieldValue::Boolean(true)); // Like is case-insensitive
    assert_eq!(ok("\"abc\" Like \"a?c\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"a1\" Like \"a#\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"ab\" Like \"a#\""), FieldValue::Boolean(false));
    assert_eq!(ok("\"m\" Like \"[m-p]\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"z\" Like \"[!m-p]\""), FieldValue::Boolean(true));
    assert_eq!(ok("\"a*b\" Like \"a[*]b\""), FieldValue::Boolean(true));
    assert_eq!(ok("Null Like \"a*\""), FieldValue::Null);
    assert_eq!(ok("\"a\" Like Null"), FieldValue::Null);
}

#[test]
fn between_in_and_is_null_follow_jet_rules() {
    assert_eq!(ok("5 Between 1 And 10"), FieldValue::Boolean(true));
    assert_eq!(ok("1 Between 2 And 3"), FieldValue::Boolean(false));
    assert_eq!(ok("Null Between 1 And 2"), FieldValue::Null);
    assert_eq!(ok("1 Between Null And 3"), FieldValue::Null);
    assert_eq!(ok("2 In (1, 2, 3)"), FieldValue::Boolean(true));
    assert_eq!(ok("4 In (1, 2, 3)"), FieldValue::Boolean(false));
    assert_eq!(ok("Null In (1, 2)"), FieldValue::Null);
    let mut scope = Scope::new();
    scope.set("x", FieldValue::Null);
    assert_eq!(ok_with("[x] Is Null", &scope), FieldValue::Boolean(true));
    assert_eq!(ok_with("[x] Is Not Null", &scope), FieldValue::Boolean(false));
}

#[test]
fn iif_and_nz_respect_null_conditions() {
    assert_eq!(ok("IIf(1 < 2, \"yes\", \"no\")"), FieldValue::Text("yes".into()));
    assert_eq!(ok("IIf(Null, 1, 2)"), FieldValue::Integer(2)); // Jet: Null condition -> false branch
    assert_eq!(ok("IIf(0, 1, 2)"), FieldValue::Integer(2));
    assert_eq!(ok("IIf(42, 1, 2)"), FieldValue::Integer(1));
    assert_eq!(ok("iif(True, 1, 2)"), FieldValue::Integer(1)); // names are case-insensitive
    assert_eq!(ok("Nz(Null)"), FieldValue::Text(String::new()));
    assert_eq!(ok("Nz(Null, 5)"), FieldValue::Integer(5));
    assert_eq!(ok("Nz(7, 9)"), FieldValue::Integer(7));
    assert_eq!(ok("Nz(Null, \"x\")"), FieldValue::Text("x".into()));
}

#[test]
fn string_functions_cover_the_documented_family() {
    assert_eq!(ok("Len(\"abc\")"), FieldValue::Integer(3));
    assert_eq!(ok("Len(Null)"), FieldValue::Null);
    assert_eq!(ok("Left(\"abcd\", 2)"), FieldValue::Text("ab".into()));
    assert_eq!(ok("Right(\"abcd\", 2)"), FieldValue::Text("cd".into()));
    assert_eq!(ok("Mid(\"abcdef\", 2, 3)"), FieldValue::Text("bcd".into()));
    assert_eq!(ok("Mid(\"abcdef\", 3)"), FieldValue::Text("cdef".into()));
    assert_eq!(ok("UCase(\"aB\")"), FieldValue::Text("AB".into()));
    assert_eq!(ok("LCase(\"aB\")"), FieldValue::Text("ab".into()));
    assert_eq!(ok("Trim(\"  a \")"), FieldValue::Text("a".into()));
    assert_eq!(ok("LTrim(\"  a\")"), FieldValue::Text("a".into()));
    assert_eq!(ok("RTrim(\"a  \")"), FieldValue::Text("a".into()));
    assert_eq!(ok("InStr(\"abcabc\", \"b\")"), FieldValue::Integer(2));
    assert_eq!(ok("Asc(\"A\")"), FieldValue::Integer(65));
    assert_eq!(ok("Chr(65)"), FieldValue::Text("A".into()));
    assert_eq!(ok("IsNull(Null)"), FieldValue::Boolean(true));
    assert_eq!(ok("IsNull(0)"), FieldValue::Boolean(false));
    assert_eq!(ok("IsNumeric(\"5\")"), FieldValue::Boolean(true));
    assert_eq!(ok("IsNumeric(\"a\")"), FieldValue::Boolean(false));
    assert_eq!(ok("IsNumeric(Null)"), FieldValue::Null);
    assert_eq!(ok("CStr(2.5)"), FieldValue::Text("2.5".into()));
    assert_eq!(ok("CStr(True)"), FieldValue::Text("True".into()));
}

#[test]
fn date_functions_read_ole_components() {
    assert_eq!(ok("Year(#2025-03-07 13:45#)"), FieldValue::Integer(2025));
    assert_eq!(ok("Month(#2025-03-07 13:45#)"), FieldValue::Integer(3));
    assert_eq!(ok("Day(#2025-03-07 13:45#)"), FieldValue::Integer(7));
    assert_eq!(ok("Hour(#2025-03-07 13:45#)"), FieldValue::Integer(13));
    assert_eq!(ok("Minute(#2025-03-07 13:45#)"), FieldValue::Integer(45));
    assert_eq!(ok("Year(#2025-03-07#)"), FieldValue::Integer(2025));
    assert_eq!(ok("DateSerial(2025, 1, 5)"), FieldValue::DateTime(45_662.0));
}

#[test]
fn format_renders_the_documented_pictures() {
    assert_eq!(ok("Format(4523.5, \"Fixed\")"), FieldValue::Text("4523.50".into()));
    assert_eq!(ok("Format(0.5, \"Percent\")"), FieldValue::Text("50.00%".into()));
    assert_eq!(
        ok("Format(1234567.891, \"Standard\")"),
        FieldValue::Text("1,234,567.89".into())
    );
    assert_eq!(
        ok("Format(#2025-01-05 13:45#, \"yyyy-mm-dd hh:nn\")"),
        FieldValue::Text("2025-01-05 13:45".into())
    );
    assert_eq!(ok("Format(3.14159, \"0.00\")"), FieldValue::Text("3.14".into()));
    assert_eq!(ok("Format(1234.5, \"#,##0\")"), FieldValue::Text("1,235".into()));
    assert_eq!(ok("Format(Null, \"Fixed\")"), FieldValue::Text(String::new()));
}

#[test]
fn math_functions_round_like_vba() {
    assert_eq!(ok("Abs(-5)"), FieldValue::Integer(5));
    assert_eq!(ok("Abs(-5.5)"), FieldValue::Float(5.5));
    assert_eq!(ok("Int(5.9)"), FieldValue::Integer(5));
    assert_eq!(ok("Int(-5.5)"), FieldValue::Integer(-6)); // floor
    assert_eq!(ok("Fix(-5.5)"), FieldValue::Integer(-5)); // truncate
    assert_eq!(ok("Round(2.5)"), FieldValue::Float(2.0)); // banker's rounding
    assert_eq!(ok("Round(3.5)"), FieldValue::Float(4.0));
    assert_eq!(ok("Round(2.567, 2)"), FieldValue::Float(2.57));
    assert_eq!(ok("Sqr(9)"), FieldValue::Float(3.0));
    is_runtime_error("Sqr(-1)", "Sqr");
}

#[test]
fn malformed_sources_are_parse_errors() {
    is_parse_error("");
    is_parse_error("1 +");
    is_parse_error("(1");
    is_parse_error("\"open");
    is_parse_error("1 @ 2");
    is_parse_error("1 2");
}

#[test]
fn runtime_errors_are_precise() {
    is_runtime_error("Foo(1)", "Foo");
    is_runtime_error("Left(\"a\")", "Left");
    is_runtime_error("Chr(70000)", "Chr");
    is_runtime_error("Year(\"nope\")", "date");
}
