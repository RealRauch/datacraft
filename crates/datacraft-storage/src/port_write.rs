//! Domain -> Jet write mapping (step 11.1).
//!
//! The mapping table and its lossiness (mirrors docs/FORMAT-JET.md
//! "Domain mapping"):
//!
//! | Domain kind | Jet column | Lossiness |
//! |---|---|---|
//! | `Text { max_len }` | variable `Text`, byte budget `2 * max_len` | none (UTF-16) |
//! | `Memo` | variable `Memo` | none (inline long-value form, documented writer divergence) |
//! | `Number(Byte/Integer/LongInteger)` | fixed `Byte`/`Int`/`LongInt` | none |
//! | `Number(Single/Double)` | fixed `Float`/`Double` | none |
//! | `Number(Decimal { scale })` | fixed `Numeric` (17 bytes), scale in `misc` | precision modeled as 28 digits |
//! | `DateTime` | fixed `DateTime` (OLE days) | none |
//! | `Currency` | fixed `Currency` (scaled 10⁻⁴) | none |
//! | `Boolean` | `Bool` (mask bit, no storage) | none |
//! | `Binary(OleObject)` | variable `Ole` | none (inline long-value form) |
//! | `Guid` | fixed `Guid` (16 bytes) | none |
//! | `AutoNumber(LongInteger)` | fixed `LongInt` with the AUTONUMBER flag; Null = engine-assigned | none (the writer maintains the TDEF counter) |
//! | `AutoNumber(ReplicationId)` | — | **`Unsupported`**: GUID AutoNumbers stay unsupported (M8 remainder) |
//! | `Binary(Attachment)` / `MultiValue(_)` / `Complex` | — | **`Unsupported`**: complex-value columns are read-only (M4) |

use datacraft_core::port::PortError;
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};

use crate::jet::tdef::flags;
use crate::jet::{JetColumn, JetType};
use crate::types::StorageValue;

/// Maps a validated table definition onto Jet column descriptors.
pub(crate) fn table_to_columns(definition: &TableDef) -> Result<Vec<JetColumn>, PortError> {
    definition.validate().map_err(|e| PortError::WriteFailed {
        detail: e.to_string(),
    })?;
    let mut columns = Vec::with_capacity(definition.fields.len());
    for (index, field) in definition.fields.iter().enumerate() {
        let (data_type, len, fixed, scale) = column_shape(&field.kind)?;
        let mut column_flags = 0u8;
        if fixed {
            column_flags |= flags::FIXED;
        }
        if !field.properties.required {
            column_flags |= flags::NULLABLE;
        }
        if matches!(field.kind, FieldKind::AutoNumber(_)) {
            // Engine-assigned: the flag drives the read mapping; the column
            // is never nullable (step 24.1, item M8).
            column_flags |= flags::AUTONUMBER;
            column_flags &= !flags::NULLABLE;
        }
        let mut column = JetColumn::new(
            &field.name,
            data_type,
            column_flags,
            u16::try_from(index).map_err(|_| PortError::WriteFailed {
                detail: "tables with more than 65535 fields are not supported".to_owned(),
            })?,
            0,
            len,
        );
        column.scale = scale;
        columns.push(column);
    }
    Ok(columns)
}

/// Jet type, storage length, fixed flag and scale of one domain kind.
fn column_shape(kind: &FieldKind) -> Result<(JetType, u16, bool, u8), PortError> {
    Ok(match kind {
        FieldKind::Text { max_len } => (JetType::Text, max_len.saturating_mul(2), false, 0),
        FieldKind::Memo => (JetType::Memo, 0, false, 0),
        FieldKind::Number(NumericKind::Byte) => (JetType::Byte, 1, true, 0),
        FieldKind::Number(NumericKind::Integer) => (JetType::Int, 2, true, 0),
        FieldKind::Number(NumericKind::LongInteger) => (JetType::LongInt, 4, true, 0),
        FieldKind::Number(NumericKind::Single) => (JetType::Float, 4, true, 0),
        FieldKind::Number(NumericKind::Double) => (JetType::Double, 8, true, 0),
        FieldKind::Number(NumericKind::Decimal { scale, .. }) => {
            (JetType::Numeric, 17, true, *scale)
        }
        FieldKind::DateTime => (JetType::DateTime, 8, true, 0),
        FieldKind::Currency => (JetType::Currency, 8, true, 0),
        FieldKind::Boolean { .. } => (JetType::Bool, 0, true, 0),
        FieldKind::Binary(BinaryKind::OleObject) => (JetType::Ole, 0, false, 0),
        FieldKind::Binary(BinaryKind::Attachment)
        | FieldKind::MultiValue(_)
        | FieldKind::Complex => {
            return Err(PortError::Unsupported {
                reason: "complex-value columns (attachments, multi-value, version history) \
                         are read-only"
                    .to_owned(),
            });
        }
        FieldKind::Guid => (JetType::Guid, 16, true, 0),
        FieldKind::AutoNumber(AutoKind::LongInteger) => (JetType::LongInt, 4, true, 0),
        FieldKind::AutoNumber(AutoKind::ReplicationId) => {
            return Err(PortError::Unsupported {
                reason: "GUID AutoNumbers (ReplicationId) are not writable — only the \
                         LongInteger flavor is maintained"
                    .to_owned(),
            });
        }
    })
}

/// Maps domain values onto the physical storage vocabulary.
pub(crate) fn to_storage_values(
    fields: &[FieldDef],
    values: &[FieldValue],
) -> Result<Vec<StorageValue>, PortError> {
    if values.len() != fields.len() {
        return Err(PortError::WriteFailed {
            detail: format!(
                "row holds {} values but the table defines {} fields",
                values.len(),
                fields.len()
            ),
        });
    }
    let mut out = Vec::with_capacity(values.len());
    for (field, value) in fields.iter().zip(values) {
        if matches!(field.kind, FieldKind::AutoNumber(_)) {
            if matches!(value, FieldValue::Null) {
                // Engine-assigned: the writer fills the table's next counter
                // value (step 24.1, item M8).
                out.push(StorageValue::Null);
                continue;
            }
            return Err(PortError::WriteFailed {
                detail: format!(
                    "field {:?}: AutoNumber values are engine-assigned — leave the value Null",
                    field.name
                ),
            });
        }
        field
            .validate_value(value)
            .map_err(|e| PortError::WriteFailed {
                detail: format!("field {:?}: {e}", field.name),
            })?;
        out.push(to_storage_value(value)?);
    }
    Ok(out)
}

/// Maps domain values onto the physical storage vocabulary for an update:
/// AutoNumber columns keep their engine-assigned value, so they are written
/// as-is (the bus guarantees the value is unchanged); everything else is
/// validated like an insert.
pub(crate) fn to_storage_values_for_update(
    fields: &[FieldDef],
    values: &[FieldValue],
) -> Result<Vec<StorageValue>, PortError> {
    if values.len() != fields.len() {
        return Err(PortError::WriteFailed {
            detail: format!(
                "row holds {} values but the table defines {} fields",
                values.len(),
                fields.len()
            ),
        });
    }
    let mut out = Vec::with_capacity(values.len());
    for (field, value) in fields.iter().zip(values) {
        if matches!(field.kind, FieldKind::AutoNumber(_)) {
            if !matches!(value, FieldValue::Integer(_)) {
                return Err(PortError::WriteFailed {
                    detail: format!(
                        "field {:?}: AutoNumber values must be integers on update",
                        field.name
                    ),
                });
            }
            out.push(to_storage_value(value)?);
            continue;
        }
        field
            .validate_value(value)
            .map_err(|e| PortError::WriteFailed {
                detail: format!("field {:?}: {e}", field.name),
            })?;
        out.push(to_storage_value(value)?);
    }
    Ok(out)
}

fn to_storage_value(value: &FieldValue) -> Result<StorageValue, PortError> {
    Ok(match value {
        FieldValue::Null => StorageValue::Null,
        FieldValue::Boolean(b) => StorageValue::Bit(*b),
        FieldValue::Integer(i) => StorageValue::Int(*i),
        FieldValue::Float(f) => StorageValue::Float(*f),
        FieldValue::Currency(c) => StorageValue::Currency(*c),
        FieldValue::DateTime(d) => StorageValue::DateTime(*d),
        FieldValue::Decimal(d) => StorageValue::Decimal(*d),
        FieldValue::Text(s) => StorageValue::Text(s.clone()),
        FieldValue::Binary(b) => StorageValue::Binary(b.clone()),
        FieldValue::Guid(g) => StorageValue::Guid(*g),
        FieldValue::Complex(_) => {
            return Err(PortError::Unsupported {
                reason: "complex values cannot be written yet — the complex-value path is \
                         read-only"
                    .to_owned(),
            });
        }
    })
}

#[cfg(test)]
#[path = "port_write_tests.rs"]
mod tests;
