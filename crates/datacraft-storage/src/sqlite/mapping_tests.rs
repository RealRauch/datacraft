use super::*;

#[test]
fn affinity_table_maps_declared_types() {
    let case = |declared: &str, pk: i64, autoincrement: bool| {
        field_kind(declared, pk, autoincrement, true)
    };
    assert_eq!(
        case("INTEGER", 1, true),
        FieldKind::AutoNumber(AutoKind::LongInteger)
    );
    assert_eq!(
        case("INTEGER", 1, false),
        FieldKind::Number(NumericKind::LongInteger)
    );
    assert_eq!(
        case("VARCHAR(40)", 0, false),
        FieldKind::Text { max_len: 40 }
    );
    assert_eq!(case("TEXT", 0, false), FieldKind::Text { max_len: 255 });
    assert_eq!(
        case("BLOB", 0, false),
        FieldKind::Binary(BinaryKind::OleObject)
    );
    assert_eq!(
        case("REAL", 0, false),
        FieldKind::Number(NumericKind::Double)
    );
    assert_eq!(
        case("NUMERIC", 0, false),
        FieldKind::Number(NumericKind::Decimal {
            precision: 28,
            scale: 0
        })
    );
    assert_eq!(
        case("BOOLEAN", 0, false),
        FieldKind::Boolean { tri_state: true }
    );
    assert_eq!(case("DATETIME", 0, false), FieldKind::DateTime);
    assert_eq!(case("CURRENCY", 0, false), FieldKind::Currency);
    assert_eq!(case("GUID", 0, false), FieldKind::Guid);
}

#[test]
fn text_budget_falls_back_to_the_access_maximum() {
    assert_eq!(text_budget("VARCHAR(12)"), 12);
    assert_eq!(text_budget("VARCHAR(999)"), 255);
    assert_eq!(text_budget("VARCHAR(0)"), 1);
    assert_eq!(text_budget("TEXT"), 255);
    assert_eq!(text_budget("VARCHAR("), 255);
}

#[test]
fn guid_text_parses_with_and_without_braces() {
    let expected = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    assert_eq!(
        parse_guid("01234567-89ab-cdef-fedc-ba9876543210"),
        Some(expected)
    );
    assert_eq!(
        parse_guid("{01234567-89ab-cdef-fedc-ba9876543210}"),
        Some(expected)
    );
    assert_eq!(parse_guid("not-a-guid"), None);
}

#[test]
fn identifiers_are_quoted_and_escaped() {
    assert_eq!(quote_ident("plain"), "\"plain\"");
    assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
}
