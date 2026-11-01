//! Localized presentation of core domain values (step 18.2, item I2).
//!
//! Core stays locale-neutral: snapshots carry enum kinds and raw values, and
//! the `inspect()` JSON is byte-identical in every language. The UI renders
//! the display text here, in the current language (ADR-4, ARCHITECTURE §15).

use datacraft_core::schema::{AutoKind, BinaryKind, FieldKind, FieldValue, NumericKind};

use crate::i18n::{t, tr};

/// Access UI type name of a field kind (design grid and field list).
pub fn field_kind_label(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Text { .. } => t("Short Text").to_owned(),
        FieldKind::Memo => t("Long Text").to_owned(),
        FieldKind::Number(numeric) => match numeric {
            NumericKind::Byte => t("Byte").to_owned(),
            NumericKind::Integer => t("Integer").to_owned(),
            NumericKind::LongInteger => t("Long Integer").to_owned(),
            NumericKind::Single => t("Single").to_owned(),
            NumericKind::Double => t("Double").to_owned(),
            NumericKind::Decimal { precision, scale } => tr(
                "Decimal ({precision}, {scale})",
                &[
                    ("precision", &precision.to_string()),
                    ("scale", &scale.to_string()),
                ],
            ),
        },
        FieldKind::DateTime => t("Date/Time").to_owned(),
        FieldKind::Currency => t("Currency").to_owned(),
        FieldKind::AutoNumber(AutoKind::LongInteger) => t("AutoNumber").to_owned(),
        FieldKind::AutoNumber(AutoKind::ReplicationId) => {
            t("AutoNumber (Replication ID)").to_owned()
        }
        FieldKind::Boolean { .. } => t("Yes/No").to_owned(),
        FieldKind::Binary(BinaryKind::OleObject) => t("OLE Object").to_owned(),
        FieldKind::Binary(BinaryKind::Attachment) => t("Attachment").to_owned(),
        FieldKind::MultiValue(_) => t("Multi-Value").to_owned(),
        FieldKind::Complex => t("Complex").to_owned(),
        FieldKind::Guid => t("GUID").to_owned(),
    }
}

/// User-renderable text for one cell. Core's locale-neutral formatting
/// carries numbers and dates; the UI adds the language for booleans, binary
/// markers and complex joins.
pub fn value_label(value: &FieldValue) -> String {
    match value {
        FieldValue::Boolean(true) => t("Yes").to_owned(),
        FieldValue::Boolean(false) => t("No").to_owned(),
        FieldValue::Binary(bytes) => tr(
            "<binary: {bytes} bytes>",
            &[("bytes", &bytes.len().to_string())],
        ),
        FieldValue::Complex(records) => records
            .iter()
            .filter_map(|record| record.scalar())
            .map(value_label)
            .collect::<Vec<_>>()
            .join(", "),
        other => datacraft_core::present::display_value(other),
    }
}
