//! Pure value rendering helpers (step 5.1, item H17; ARCHITECTURE §8).
//!
//! The Access display conventions shared by the UI, the criteria parser's
//! `Like` matcher and the expression service's `Format`: booleans render
//! `Yes`/`No`, currency trims trailing scaled zeros, OLE dates render
//! `YYYY-MM-DD HH:MM`, GUIDs the canonical hyphenated form. Core's
//! `present` module re-exports these paths for its clients.

use crate::date::civil_from_days;
use crate::field::FieldKind;
use crate::value::FieldValue;

/// Access UI type name of a field kind (the design grid's Data Type
/// column).
pub fn field_kind_display(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Text { .. } => "Short Text".to_owned(),
        FieldKind::Memo => "Long Text".to_owned(),
        FieldKind::Number(numeric) => match numeric {
            crate::NumericKind::Byte => "Byte".to_owned(),
            crate::NumericKind::Integer => "Integer".to_owned(),
            crate::NumericKind::LongInteger => "Long Integer".to_owned(),
            crate::NumericKind::Single => "Single".to_owned(),
            crate::NumericKind::Double => "Double".to_owned(),
            crate::NumericKind::Decimal { precision, scale } => {
                format!("Decimal ({precision}, {scale})")
            }
        },
        FieldKind::DateTime => "Date/Time".to_owned(),
        FieldKind::Currency => "Currency".to_owned(),
        FieldKind::AutoNumber(crate::AutoKind::LongInteger) => "AutoNumber".to_owned(),
        FieldKind::AutoNumber(crate::AutoKind::ReplicationId) => {
            "AutoNumber (Replication ID)".to_owned()
        }
        FieldKind::Boolean { .. } => "Yes/No".to_owned(),
        FieldKind::Binary(crate::BinaryKind::OleObject) => "OLE Object".to_owned(),
        FieldKind::Binary(crate::BinaryKind::Attachment) => "Attachment".to_owned(),
        FieldKind::MultiValue(_) => "Multi-Value".to_owned(),
        FieldKind::Complex => "Complex".to_owned(),
        FieldKind::Guid => "GUID".to_owned(),
    }
}

/// User-renderable text for one cell (Access-like conventions: `Yes`/`No`
/// booleans, currency with up to 4 scaled decimals, OLE dates as
/// `YYYY-MM-DD HH:MM`).
pub fn display_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => String::new(),
        FieldValue::Boolean(true) => "Yes".to_owned(),
        FieldValue::Boolean(false) => "No".to_owned(),
        FieldValue::Integer(i) => i.to_string(),
        FieldValue::Float(f) => f.to_string(),
        FieldValue::Currency(scaled) => display_currency(*scaled),
        FieldValue::DateTime(days) => display_ole_datetime(*days),
        FieldValue::Decimal(scaled) => scaled.to_string(),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Binary(bytes) => format!("<binary: {} bytes>", bytes.len()),
        FieldValue::Guid(bytes) => display_guid(bytes),
        // Complex cells render as the joined scalar of their sub-records:
        // attachment file names, multi-value elements, version-history text.
        FieldValue::Complex(records) => records
            .iter()
            .filter_map(|record| record.scalar())
            .map(display_value)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Currency is a scaled 64-bit integer (factor 10^-4); trailing zeros trim.
fn display_currency(scaled: i64) -> String {
    let sign = if scaled < 0 { "-" } else { "" };
    let abs = scaled.unsigned_abs();
    let whole = abs / 10_000;
    let frac = abs % 10_000;
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let mut frac = format!("{frac:04}");
        while frac.ends_with('0') {
            frac.pop();
        }
        format!("{sign}{whole}.{frac}")
    }
}

/// OLE Automation date (days since 1899-12-30; 25569 days to the Unix
/// epoch) rendered via the civil-from-days algorithm; shared with the
/// expression service's `Format` ("General Date").
pub fn display_ole_datetime(days: f64) -> String {
    let whole = days.floor();
    let mut minutes = ((days - whole) * 1440.0).round() as i64;
    let mut day = whole as i64;
    if minutes >= 1440 {
        minutes -= 1440;
        day += 1;
    }
    let (year, month, mday) = civil_from_days(day - 25_569);
    format!(
        "{year:04}-{month:02}-{mday:02} {:02}:{:02}",
        minutes / 60,
        minutes % 60
    )
}

/// 16 bytes as the canonical 8-4-4-4-12 lowercase hex form.
fn display_guid(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComplexField, ComplexRecord};

    /// The minutes-to-next-day rollover keeps the wall clock exact even
    /// for float days that round into the next whole day. The OLE epoch
    /// is 1899-12-30.
    #[test]
    fn ole_datetime_rolls_over_rounded_minutes() {
        // 1.99999… days rounds to 1440 minutes on OLE day 1 → next day
        // 00:00 (1900-01-01, since OLE day 1 is 1899-12-31).
        assert_eq!(display_ole_datetime(1.999_999_999_9), "1900-01-01 00:00");
        // Half a day stays on its own day.
        assert_eq!(display_ole_datetime(0.5), "1899-12-30 12:00");
    }

    /// Every family renders in its Access convention; complex cells join
    /// the scalars of their sub-records.
    #[test]
    fn display_value_covers_every_family() {
        assert_eq!(display_value(&FieldValue::Null), "");
        assert_eq!(display_value(&FieldValue::Boolean(true)), "Yes");
        assert_eq!(display_value(&FieldValue::Boolean(false)), "No");
        assert_eq!(display_value(&FieldValue::Integer(-3)), "-3");
        assert_eq!(display_value(&FieldValue::Float(1.5)), "1.5");
        assert_eq!(display_value(&FieldValue::Currency(12_500)), "1.25");
        assert_eq!(display_value(&FieldValue::Currency(-100)), "-0.01");
        assert_eq!(display_value(&FieldValue::Decimal(7)), "7");
        assert_eq!(display_value(&FieldValue::Text("Ada".into())), "Ada");
        assert_eq!(
            display_value(&FieldValue::Binary(vec![1, 2])),
            "<binary: 2 bytes>"
        );
        assert_eq!(
            display_value(&FieldValue::Guid([0xab; 16])),
            "abababab-abab-abab-abab-abababababab"
        );
        let complex = FieldValue::Complex(vec![
            ComplexRecord {
                fields: vec![ComplexField {
                    name: "FileName".into(),
                    value: FieldValue::Text("a.txt".into()),
                }],
            },
            ComplexRecord {
                fields: vec![ComplexField {
                    name: "FileName".into(),
                    value: FieldValue::Text("b.txt".into()),
                }],
            },
        ]);
        assert_eq!(display_value(&complex), "a.txt, b.txt");
    }

    /// Every kind renders its Access UI name.
    #[test]
    fn field_kind_display_covers_every_kind() {
        use crate::FieldKind;
        let cases = [
            (FieldKind::Text { max_len: 1 }, "Short Text"),
            (FieldKind::Memo, "Long Text"),
            (
                FieldKind::Number(crate::NumericKind::Decimal {
                    precision: 10,
                    scale: 2,
                }),
                "Decimal (10, 2)",
            ),
            (FieldKind::DateTime, "Date/Time"),
            (FieldKind::Currency, "Currency"),
            (
                FieldKind::AutoNumber(crate::AutoKind::ReplicationId),
                "AutoNumber (Replication ID)",
            ),
            (FieldKind::Boolean { tri_state: true }, "Yes/No"),
            (
                FieldKind::Binary(crate::BinaryKind::Attachment),
                "Attachment",
            ),
            (
                FieldKind::MultiValue(crate::MultiValueKind::Text { max_len: 5 }),
                "Multi-Value",
            ),
            (FieldKind::Complex, "Complex"),
            (FieldKind::Guid, "GUID"),
        ];
        for (kind, expected) in cases {
            assert_eq!(field_kind_display(&kind), expected);
        }
    }
}
