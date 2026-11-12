//! SQLite value conversion (split from `mapping.rs` for the file-size
//! gate in phase 73).

use datacraft_core::schema::{FieldKind, FieldValue, NumericKind};
use rusqlite::types::ValueRef;

/// Domain value -> SQLite parameter, keeping the column's storage class.
pub(crate) fn sql_value(kind: &FieldKind, value: &FieldValue) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        FieldValue::Null => Sql::Null,
        FieldValue::Boolean(b) => Sql::Integer(i64::from(*b)),
        FieldValue::Integer(i) => Sql::Integer(*i),
        FieldValue::Float(f) => Sql::Real(*f),
        FieldValue::Currency(c) => Sql::Integer(*c),
        FieldValue::DateTime(days) => Sql::Real(*days),
        FieldValue::Decimal(scaled) => {
            let scale = match kind {
                FieldKind::Number(NumericKind::Decimal { scale, .. }) => u32::from(*scale),
                _ => 0,
            };
            if scale == 0 {
                Sql::Integer(i64::try_from(*scaled).unwrap_or(i64::MAX))
            } else {
                Sql::Real(*scaled as f64 / decimal_factor(scale) as f64)
            }
        }
        FieldValue::Text(text) => Sql::Text(text.clone()),
        FieldValue::Binary(bytes) => Sql::Blob(bytes.clone()),
        FieldValue::Guid(bytes) => Sql::Blob(bytes.to_vec()),
        // Defensive: SQLite tables never carry complex kinds; the display
        // text keeps the mapping total.
        FieldValue::Complex(_) => Sql::Text(datacraft_core::present::display_value(value)),
    }
}

/// Converts one stored cell using the column's domain kind (booleans,
/// currency, decimal scale and GUID blobs need the schema context).
pub(crate) fn to_field_value(kind: &FieldKind, value: ValueRef<'_>) -> FieldValue {
    match value {
        ValueRef::Null => FieldValue::Null,
        ValueRef::Integer(i) => match kind {
            FieldKind::Boolean { .. } => FieldValue::Boolean(i != 0),
            FieldKind::Currency => FieldValue::Currency(i),
            FieldKind::DateTime => FieldValue::DateTime(i as f64),
            FieldKind::Number(NumericKind::Decimal { scale, .. }) => {
                FieldValue::Decimal(i128::from(i) * decimal_factor(u32::from(*scale)))
            }
            _ => FieldValue::Integer(i),
        },
        ValueRef::Real(f) => match kind {
            FieldKind::DateTime => FieldValue::DateTime(f),
            FieldKind::Number(NumericKind::Decimal { scale, .. }) => {
                FieldValue::Decimal((f * decimal_factor(u32::from(*scale)) as f64).round() as i128)
            }
            _ => FieldValue::Float(f),
        },
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).into_owned();
            match kind {
                FieldKind::Guid => {
                    parse_guid(&text).map_or(FieldValue::Text(text), FieldValue::Guid)
                }
                _ => FieldValue::Text(text),
            }
        }
        ValueRef::Blob(bytes) => match kind {
            FieldKind::Guid => <[u8; 16]>::try_from(bytes)
                .map_or_else(|_| FieldValue::Binary(bytes.to_vec()), FieldValue::Guid),
            _ => FieldValue::Binary(bytes.to_vec()),
        },
    }
}

/// 10^scale as an exact integer factor for decimal scaling.
pub(crate) fn decimal_factor(scale: u32) -> i128 {
    10i128.pow(scale.min(28))
}

/// Canonical 8-4-4-4-12 hex GUID text (with or without braces).
pub(crate) fn parse_guid(text: &str) -> Option<[u8; 16]> {
    let hex: String = text
        .trim_matches(|c| c == '{' || c == '}')
        .chars()
        .filter(|c| *c != '-')
        .collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}
