//! Scalar cell decoding and the null-mask-adjacent storage rules (split
//! from `row.rs` for the file-size gate).
//!
//! Fixed-width scalars index exact widths; the length and finiteness checks
//! live here so a hostile descriptor or value cannot reach the readers.

use crate::error::StorageError;
use crate::jet::tdef::{JetColumn, JetType};
use crate::jet::text::decode_text;
use crate::types::StorageValue;

pub(crate) fn decode_bytes(
    bytes: &[u8],
    col: &JetColumn,
    jet3: bool,
) -> Result<StorageValue, StorageError> {
    // A hostile descriptor length must not reach the fixed-width readers:
    // they index exact widths (Guid copies into `[u8; 16]`).
    if let Some(needed) = storage_width(col.data_type)
        && bytes.len() != needed
    {
        return Err(StorageError::Corrupt {
            context: "row".into(),
            detail: format!(
                "fixed column {} ({}) holds {} bytes, the type requires {needed}",
                col.column_id,
                col.name,
                bytes.len()
            ),
        });
    }
    Ok(match col.data_type {
        JetType::Bool => unreachable!("booleans are resolved from the null mask"),
        JetType::Byte => StorageValue::Int(i16::from(bytes[0]) as i64),
        JetType::Int => StorageValue::Int(i16::from_le_bytes([bytes[0], bytes[1]]) as i64),
        JetType::LongInt => {
            StorageValue::Int(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64)
        }
        JetType::Currency => StorageValue::Currency(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        JetType::Float => {
            let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64;
            finite_float(value, col)?
        }
        JetType::Double => {
            let value = f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            finite_float(value, col)?
        }
        JetType::DateTime => StorageValue::DateTime(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        JetType::Binary | JetType::Ole | JetType::LongBinary | JetType::Complex | JetType::Memo => {
            StorageValue::Binary(bytes.to_vec())
        }
        JetType::Text => StorageValue::Text(decode_text(bytes, col.compressed_text, jet3)),
        JetType::Guid => {
            let mut guid = [0u8; 16];
            guid.copy_from_slice(bytes);
            StorageValue::Guid(guid)
        }
        JetType::Numeric => decode_numeric(bytes),
    })
}

/// Floating-point cell value. The domain model (and every wire
/// serialization) requires finite floats — the write path already rejects
/// non-finite values, so the read path refuses them with the same rule
/// instead of smuggling a value `serde_json` cannot represent (M28).
fn finite_float(value: f64, col: &JetColumn) -> Result<StorageValue, StorageError> {
    if value.is_finite() {
        return Ok(StorageValue::Float(value));
    }
    Err(StorageError::Corrupt {
        context: "row".into(),
        detail: format!(
            "column {} ({}) stores the non-finite number {value}; DataCraft values require \
             finite floats",
            col.column_id, col.name
        ),
    })
}

/// Exact storage width of a fixed-width scalar type; `None` for the
/// variable families (text, binary, memo/OLE) whose slot length is data.
fn storage_width(data_type: JetType) -> Option<usize> {
    Some(match data_type {
        JetType::Byte => 1,
        JetType::Int => 2,
        JetType::LongInt | JetType::Float => 4,
        JetType::Currency | JetType::Double | JetType::DateTime => 8,
        JetType::Guid => 16,
        JetType::Numeric => 17,
        JetType::Bool
        | JetType::Binary
        | JetType::Text
        | JetType::Ole
        | JetType::Memo
        | JetType::Complex
        | JetType::LongBinary => return None,
    })
}

/// 96-bit scaled decimal in 17 bytes: sign byte (0x80 = positive) followed by
/// the big-endian magnitude. Scale is applied at the logical layer (Phase 3);
/// the sign/magnitude layout is verified against the oracle in step 2.4.
fn decode_numeric(bytes: &[u8]) -> StorageValue {
    let positive = bytes[0] & 0x80 != 0;
    let mut magnitude: u128 = 0;
    for &b in &bytes[1..17] {
        magnitude = (magnitude << 8) | b as u128;
    }
    StorageValue::Decimal(if positive {
        magnitude as i128
    } else {
        -(magnitude as i128)
    })
}
