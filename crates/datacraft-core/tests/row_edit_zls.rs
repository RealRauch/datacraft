//! Access zero-length-string contract for the shared row parser (phase 64,
//! M68; ADR-8). A blank entry is `Null` unless `Required` and
//! `AllowZeroLength` are both set; the literal `""` stores a zero-length
//! string only where the field allows it.

use datacraft_core::row_edit::{parse_field_value, row_values};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind};

fn text(name: &str, required: bool, allow_zero_length: bool) -> FieldDef {
    let mut field = FieldDef::new(name, FieldKind::Text { max_len: 50 });
    field.properties.required = required;
    field.properties.allow_zero_length = allow_zero_length;
    field
}

#[test]
fn a_blank_text_entry_is_null_by_default() {
    let field = text("name", false, false);
    assert_eq!(parse_field_value(&field, ""), Ok(FieldValue::Null));
    assert_eq!(parse_field_value(&field, "   "), Ok(FieldValue::Null));
}

#[test]
fn a_blank_required_zero_length_field_stores_a_zero_length_string() {
    let field = text("name", true, true);
    assert_eq!(
        parse_field_value(&field, ""),
        Ok(FieldValue::Text(String::new()))
    );
}

#[test]
fn the_literal_quotes_store_a_zero_length_string_when_allowed() {
    let field = text("name", false, true);
    assert_eq!(
        parse_field_value(&field, "\"\""),
        Ok(FieldValue::Text(String::new()))
    );
}

#[test]
fn the_literal_quotes_are_rejected_when_zero_length_is_not_allowed() {
    let field = text("name", false, false);
    let reason = parse_field_value(&field, "\"\"").expect_err("zero length is rejected");
    assert!(reason.contains("zero-length"), "{reason}");
}

#[test]
fn a_blank_non_text_entry_stays_null() {
    let field = FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger));
    assert_eq!(parse_field_value(&field, ""), Ok(FieldValue::Null));
}

#[test]
fn row_values_applies_the_rule_per_field() {
    let fields = vec![text("name", true, true), text("note", false, false)];
    let values = row_values(&fields, &[String::new(), String::new()]).expect("both cells parse");
    assert_eq!(
        values,
        vec![FieldValue::Text(String::new()), FieldValue::Null]
    );
}
