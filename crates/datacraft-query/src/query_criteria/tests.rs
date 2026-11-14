//! Predicate parsing/matching pins (step 43.1, item T9): family-safe
//! comparison arms, Like gating and the numeric-literal coercions.

use datacraft_types::{AutoKind, BinaryKind, NumericKind, TableDef};

use super::*;

fn kind_of(name: &str, kind: FieldKind) -> TableDef {
    TableDef::new(name, vec![FieldDef::new("v", kind)])
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
    let predicate = parse_predicate("like \"hamburg\"", "v", column).expect("like parses");
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
        let rendered = error.to_string();
        assert!(
            rendered.contains("outside the supported subset")
                && rendered.contains(&format!("{criteria:?}")),
            "{criteria}: {rendered}"
        );
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

/// Every comparison predicate pins its ordering arm, and Like on a
/// non-text column refuses precisely.
#[test]
fn predicates_pin_their_ordering_arms_and_like_refuses_non_text() {
    let table = kind_of(
        "t",
        datacraft_types::FieldKind::Number(datacraft_types::NumericKind::LongInteger),
    );
    let column = &table.fields[0];
    // Every predicate is evaluated against the literal's own value 2:
    // equal/inclusive bounds match, exclusive ones do not.
    let cases = [
        (parse_predicate("=2", "v", column), true),
        (parse_predicate("<>2", "v", column), false),
        (parse_predicate("<2", "v", column), false),
        (parse_predicate("<=2", "v", column), true),
        (parse_predicate(">2", "v", column), false),
        (parse_predicate(">=2", "v", column), true),
    ];
    for (predicate, expected) in cases {
        let predicate = predicate.expect("parses");
        assert_eq!(
            predicate.matches(&FieldValue::Integer(2)),
            expected,
            "{predicate:?}"
        );
    }
    let error =
        parse_predicate("Like \"2\"", "v", column).expect_err("Like on a number column refuses");
    assert!(error.to_string().contains("text columns"), "{error}");
}

/// Step 70.1 (item H58): a bare wildcard pattern is an implicit Like on a
/// text column (Access filter semantics); an explicit `=` stays literal.
#[test]
fn wildcard_patterns_are_an_implicit_like_on_text_columns() {
    let table = kind_of("t", FieldKind::Text { max_len: 50 });
    let column = &table.fields[0];

    let predicate = parse_predicate("*a*", "v", column).expect("wildcard parses");
    assert!(
        predicate.matches(&FieldValue::Text("Mara".into())),
        "{predicate:?}"
    );
    assert!(predicate.matches(&FieldValue::Text("Lena".into())));
    assert!(!predicate.matches(&FieldValue::Text("Tom".into())));

    let question = parse_predicate("Le?a", "v", column).expect("question mark parses");
    assert!(question.matches(&FieldValue::Text("Lena".into())));
    assert!(!question.matches(&FieldValue::Text("Lea".into())));

    let digit = parse_predicate("A##", "v", column).expect("hash parses");
    assert!(digit.matches(&FieldValue::Text("A42".into())));
    assert!(!digit.matches(&FieldValue::Text("A4".into())));

    let literal = parse_predicate("= *a*", "v", column).expect("explicit equality parses");
    assert!(!literal.matches(&FieldValue::Text("Mara".into())));
    assert!(literal.matches(&FieldValue::Text("*a*".into())));
}
