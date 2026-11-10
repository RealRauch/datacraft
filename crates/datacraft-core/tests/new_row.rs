//! Datasheet new-row contract (step 11.7).
//!
//! The datasheet's new-record row collects one text cell per column; core
//! turns those strings into a typed positional row and the command bus
//! validates it. Parsing is strict and names the field — a mistyped cell
//! never becomes a silent Null (LESSONS #4).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{MockPort, RowRange};
use datacraft_core::row_edit::{parse_value, row_values};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};

fn all_kinds_table() -> TableDef {
    TableDef::new(
        "Typed",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 20 }),
            FieldDef::new("note", FieldKind::Memo),
            FieldDef::new("small", FieldKind::Number(NumericKind::Byte)),
            FieldDef::new("big", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("ratio", FieldKind::Number(NumericKind::Double)),
            FieldDef::new(
                "amount",
                FieldKind::Number(NumericKind::Decimal {
                    precision: 6,
                    scale: 2,
                }),
            ),
            FieldDef::new("price", FieldKind::Currency),
            FieldDef::new("when", FieldKind::DateTime),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("tag", FieldKind::Guid),
            FieldDef::new("photo", FieldKind::Binary(BinaryKind::OleObject)),
        ],
    )
}

fn core_with_table() -> Core {
    let mut port = MockPort::with_tables(vec![all_kinds_table()]);
    port.with_row_data("Typed", Vec::new());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("memory.mock"),
        password: None,
        source: None,
    });
    core
}

fn cells() -> Vec<String> {
    vec![
        String::new(),                                 // AutoNumber stays empty
        "Ada".into(),                                  // name
        "first row".into(),                            // note
        "255".into(),                                  // small (Byte)
        "-2147483648".into(),                          // big (Long Integer)
        "1.5".into(),                                  // ratio
        "12.34".into(),                                // amount (Decimal 6,2)
        "99.9999".into(),                              // price (Currency)
        "1970-01-01 12:00".into(),                     // when
        "yes".into(),                                  // active
        "00112233-4455-6677-8899-aabbccddeeff".into(), // tag
        String::new(),                                 // photo stays Null
    ]
}

#[test]
fn empty_cells_are_null_for_every_kind() {
    for kind in [
        FieldKind::Text { max_len: 5 },
        FieldKind::Memo,
        FieldKind::Number(NumericKind::LongInteger),
        FieldKind::Number(NumericKind::Double),
        FieldKind::Number(NumericKind::Decimal {
            precision: 4,
            scale: 2,
        }),
        FieldKind::DateTime,
        FieldKind::Currency,
        FieldKind::Boolean { tri_state: false },
        FieldKind::Guid,
        FieldKind::Binary(BinaryKind::Attachment),
    ] {
        assert_eq!(parse_value(&kind, ""), Ok(FieldValue::Null), "{kind:?}");
        assert_eq!(parse_value(&kind, "   "), Ok(FieldValue::Null), "{kind:?}");
    }
    // AutoNumber cells stay empty; the engine assigns the value.
    assert_eq!(
        parse_value(&FieldKind::AutoNumber(AutoKind::LongInteger), ""),
        Ok(FieldValue::Null)
    );
}

#[test]
fn text_and_memo_cells_keep_their_text() {
    assert_eq!(
        parse_value(&FieldKind::Text { max_len: 10 }, " Ada "),
        Ok(FieldValue::Text(" Ada ".into()))
    );
    assert_eq!(
        parse_value(&FieldKind::Memo, "line one\nline two"),
        Ok(FieldValue::Text("line one\nline two".into()))
    );
}

#[test]
fn integer_cells_parse_and_respect_the_kind_range() {
    let byte = FieldKind::Number(NumericKind::Byte);
    assert_eq!(parse_value(&byte, "255"), Ok(FieldValue::Integer(255)));
    let reason = parse_value(&byte, "256").expect_err("range");
    assert!(reason.contains("255"), "{reason}");
    let reason = parse_value(&byte, "abc").expect_err("not a number");
    assert!(reason.contains("integer"), "{reason}");
    assert_eq!(
        parse_value(&FieldKind::Number(NumericKind::LongInteger), "-42"),
        Ok(FieldValue::Integer(-42))
    );
}

#[test]
fn floating_point_cells_reject_non_finite_input() {
    let double = FieldKind::Number(NumericKind::Double);
    assert_eq!(parse_value(&double, "1.5"), Ok(FieldValue::Float(1.5)));
    let reason = parse_value(&double, "NaN").expect_err("not finite");
    assert!(reason.contains("finite"), "{reason}");
}

#[test]
fn decimal_cells_scale_and_reject_excess_precision() {
    let kind = FieldKind::Number(NumericKind::Decimal {
        precision: 6,
        scale: 2,
    });
    assert_eq!(parse_value(&kind, "12.34"), Ok(FieldValue::Decimal(1234)));
    assert_eq!(parse_value(&kind, "-0.5"), Ok(FieldValue::Decimal(-50)));
    assert_eq!(parse_value(&kind, "7"), Ok(FieldValue::Decimal(700)));
    let reason = parse_value(&kind, "1.234").expect_err("too many places");
    assert!(reason.contains("2 decimal places"), "{reason}");
    let reason = parse_value(&kind, "12345.67").expect_err("too many digits");
    assert!(reason.contains("6 digits"), "{reason}");
}

#[test]
fn malformed_number_cells_are_rejected() {
    let decimal = FieldKind::Number(NumericKind::Decimal {
        precision: 6,
        scale: 2,
    });
    assert!(parse_value(&decimal, "abc").is_err());
    assert!(parse_value(&decimal, "1.2.3").is_err());
    assert_eq!(parse_value(&decimal, ".5"), Ok(FieldValue::Decimal(50)));
    let whole = FieldKind::Number(NumericKind::Decimal {
        precision: 6,
        scale: 0,
    });
    assert_eq!(parse_value(&whole, "7"), Ok(FieldValue::Decimal(7)));
}

#[test]
fn malformed_time_cells_are_rejected() {
    let kind = FieldKind::DateTime;
    for text in [
        "2020-01-01 12",
        "2020-01-01 12:60",
        "2020-01-01 12:00:60",
        "2020-01-01 12:00:00:00",
    ] {
        assert!(parse_value(&kind, text).is_err(), "{text}");
    }
}

#[test]
fn currency_cells_scale_to_four_decimals() {
    assert_eq!(
        parse_value(&FieldKind::Currency, "12.34"),
        Ok(FieldValue::Currency(123_400))
    );
    assert_eq!(
        parse_value(&FieldKind::Currency, "-0.0001"),
        Ok(FieldValue::Currency(-1))
    );
    let reason = parse_value(&FieldKind::Currency, "1.00001").expect_err("places");
    assert!(reason.contains("4 decimal places"), "{reason}");
}

#[test]
fn date_cells_use_the_ole_automation_epoch() {
    let kind = FieldKind::DateTime;
    assert_eq!(
        parse_value(&kind, "1899-12-30"),
        Ok(FieldValue::DateTime(0.0))
    );
    assert_eq!(
        parse_value(&kind, "1900-01-01"),
        Ok(FieldValue::DateTime(2.0))
    );
    assert_eq!(
        parse_value(&kind, "1970-01-01"),
        Ok(FieldValue::DateTime(25_569.0))
    );
    assert_eq!(
        parse_value(&kind, "2000-01-01"),
        Ok(FieldValue::DateTime(36_526.0))
    );
    assert_eq!(
        parse_value(&kind, "1899-12-30 12:00"),
        Ok(FieldValue::DateTime(0.5))
    );
    assert_eq!(
        parse_value(&kind, "1899-12-30T06:30:30"),
        Ok(FieldValue::DateTime(
            (6.0 * 3600.0 + 30.0 * 60.0 + 30.0) / 86_400.0
        ))
    );
}

#[test]
fn date_cells_reject_impossible_dates_and_times() {
    let kind = FieldKind::DateTime;
    for text in ["2021-02-29", "2020-13-01", "2020-01-32", "2020-01-01 25:00"] {
        let reason = parse_value(&kind, text).expect_err(text);
        assert!(
            reason.contains("date") || reason.contains("time"),
            "{text}: {reason}"
        );
    }
    assert!(parse_value(&kind, "2020-02-29").is_ok(), "leap day");
    assert!(parse_value(&kind, "tomorrow").is_err());
}

#[test]
fn boolean_cells_accept_the_access_spellings() {
    let kind = FieldKind::Boolean { tri_state: false };
    for text in ["true", "TRUE", "yes", "y", "1", "on", "-1"] {
        assert_eq!(
            parse_value(&kind, text),
            Ok(FieldValue::Boolean(true)),
            "{text}"
        );
    }
    for text in ["false", "no", "n", "0", "off"] {
        assert_eq!(
            parse_value(&kind, text),
            Ok(FieldValue::Boolean(false)),
            "{text}"
        );
    }
    assert!(parse_value(&kind, "maybe").is_err());
}

#[test]
fn guid_cells_accept_braced_and_plain_forms() {
    let expected = FieldValue::Guid([
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ]);
    assert_eq!(
        parse_value(&FieldKind::Guid, "00112233-4455-6677-8899-aabbccddeeff"),
        Ok(expected.clone())
    );
    assert_eq!(
        parse_value(&FieldKind::Guid, "{00112233-4455-6677-8899-AABBCCDDEEFF}"),
        Ok(expected)
    );
    assert!(parse_value(&FieldKind::Guid, "00112233").is_err());
    assert!(parse_value(&FieldKind::Guid, "zz112233-4455-6677-8899-aabbccddeeff").is_err());
}

#[test]
fn binary_and_autonumber_cells_refuse_user_input() {
    let reason = parse_value(&FieldKind::Binary(BinaryKind::OleObject), "abc").expect_err("binary");
    assert!(reason.contains("binary"), "{reason}");
    let reason =
        parse_value(&FieldKind::AutoNumber(AutoKind::LongInteger), "7").expect_err("autonumber");
    assert!(reason.contains("engine-assigned"), "{reason}");
}

#[test]
fn row_values_parse_the_full_positional_row() {
    let values = row_values(&all_kinds_table().fields, &cells()).expect("valid row");
    assert_eq!(values[0], FieldValue::Null);
    assert_eq!(values[1], FieldValue::Text("Ada".into()));
    assert_eq!(values[3], FieldValue::Integer(255));
    assert_eq!(values[6], FieldValue::Decimal(1234));
    assert_eq!(values[7], FieldValue::Currency(999_999));
    assert_eq!(values[8], FieldValue::DateTime(25_569.5));
    assert_eq!(values[9], FieldValue::Boolean(true));
    assert_eq!(
        values[10],
        FieldValue::Guid([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff
        ])
    );
    assert_eq!(values[11], FieldValue::Null);
}

#[test]
fn row_values_name_the_offending_field_and_arity() {
    let table = all_kinds_table();
    let mut broken = cells();
    broken[3] = "300".into();
    let reason = row_values(&table.fields, &broken).expect_err("range");
    assert!(reason.contains("\"small\""), "{reason}");
    assert!(reason.contains("255"), "{reason}");

    let reason = row_values(&table.fields, &cells()[..2]).expect_err("arity");
    assert!(reason.contains("expected 12 cells"), "{reason}");
    assert!(reason.contains("found 2"), "{reason}");
}

#[test]
fn a_parsed_row_inserts_through_the_bus_and_reads_back() {
    let mut core = core_with_table();
    let table = all_kinds_table();
    let values = row_values(&table.fields, &cells()).expect("valid row");
    assert_eq!(
        core.dispatch(Command::InsertRow {
            table: "Typed".into(),
            values: values.clone(),
        }),
        vec![Event::RowInserted {
            table: "Typed".into()
        }]
    );
    let rows = core
        .read_rows("Typed", RowRange { start: 0, len: 10 })
        .expect("rows readable");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1], values[1]);
    assert_eq!(rows[0][9], values[9]);
}

#[test]
fn core_exposes_the_table_definition_for_the_row_editor() {
    let core = core_with_table();
    let def = core.table_definition("typed").expect("case-insensitive");
    assert_eq!(def.name, "Typed");
    assert_eq!(def.fields.len(), 12);
    assert!(core.table_definition("Ghost").is_none());
}
