//! Jet4 row and TDEF encoders (step 10.2, item H9) — inverses of the
//! decoders in `row.rs`/`tdef.rs`, same fact base (`docs/FORMAT-JET.md`).
//!
//! Encoder policies (documented divergences from real writers):
//! - Memo/OLE values use the **inline** form (`0x8000_0000 | len`) — LVAL
//!   page chains are not written yet.
//! - Text is stored uncompressed (UCS-2LE) unless the column is flagged
//!   compressed, in which case the `FF FE` marker form is produced.
//! - Numeric encodes the raw unscaled 96-bit magnitude (scale application
//!   stays at the logical layer, symmetric to the decoder).

use crate::error::StorageError;
use crate::jet::row::fixed_len;
use crate::jet::tdef::{JetColumn, JetTableDef, JetType, flags};
use crate::types::StorageValue;

/// Days between 1899-12-30 (OLE automation epoch) and 1970-01-01.
const UNIX_EPOCH_OLE_OFFSET: f64 = 25569.0;

/// Current time as an OLE automation date (days since 1899-12-30).
pub fn now_as_ole_date() -> f64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    UNIX_EPOCH_OLE_OFFSET + secs / 86_400.0
}

/// Encodes one row against its table definition (inverse of
/// [`crate::jet::decode_row`]): `[num_cols][fixed][var][eod][var_table
/// reversed][var_len][null mask]`, mask bits follow the column id.
pub fn encode_row(values: &[StorageValue], def: &JetTableDef) -> Result<Vec<u8>, StorageError> {
    if values.len() != def.columns.len() {
        return Err(StorageError::Unsupported {
            reason: format!(
                "row holds {} values but the table defines {} columns",
                values.len(),
                def.columns.len()
            ),
        });
    }
    let num_cols = def.columns.len();
    let fixed_size = def
        .columns
        .iter()
        .filter(|c| c.is_fixed() && c.data_type != JetType::Bool)
        .map(|c| c.fixed_offset as usize + fixed_len(c))
        .max()
        .unwrap_or(0);

    let mut fixed = vec![0u8; fixed_size];
    let mut var_data: Vec<u8> = Vec::new();
    let mut var_starts: Vec<usize> = Vec::new();
    let mut mask = vec![0u8; num_cols.div_ceil(8)];

    for (col, value) in def.columns.iter().zip(values) {
        let id = col.column_id as usize;
        if col.data_type == JetType::Bool {
            // The boolean value IS the mask bit.
            let StorageValue::Bit(bit) = value else {
                return Err(type_error(col, value));
            };
            if *bit {
                mask[id / 8] |= 1 << (id % 8);
            }
            continue;
        }
        if matches!(value, StorageValue::Null) {
            continue; // mask bit stays 0
        }
        mask[id / 8] |= 1 << (id % 8);
        if col.is_fixed() {
            write_fixed(&mut fixed, col, value)?;
        } else {
            var_starts.push(var_data.len());
            append_variable(&mut var_data, col, value)?;
        }
    }

    let eod = 2 + fixed_size + var_data.len();
    let mut row = Vec::with_capacity(eod + 2 + var_starts.len() * 2 + 2 + mask.len());
    row.extend_from_slice(&(num_cols as u16).to_le_bytes());
    row.extend_from_slice(&fixed);
    row.extend_from_slice(&var_data);
    row.extend_from_slice(&(eod as u16).to_le_bytes());
    // var_table is stored in reverse order (entry 0 = last variable column).
    for start in var_starts.iter().rev() {
        let at = start + 2 + fixed_size;
        row.extend_from_slice(&(at as u16).to_le_bytes());
    }
    row.extend_from_slice(&(var_starts.len() as u16).to_le_bytes());
    row.extend_from_slice(&mask);
    Ok(row)
}

fn type_error(col: &JetColumn, value: &StorageValue) -> StorageError {
    StorageError::Unsupported {
        reason: format!(
            "column {} ({}) cannot hold {:?}",
            col.name, col.column_id, value
        ),
    }
}

fn write_fixed(
    fixed: &mut [u8],
    col: &JetColumn,
    value: &StorageValue,
) -> Result<(), StorageError> {
    let at = col.fixed_offset as usize;
    let len = fixed_len(col);
    let slot = fixed
        .get_mut(at..at + len)
        .ok_or_else(|| StorageError::Corrupt {
            context: "row encode".into(),
            detail: format!("fixed offset {} exceeds the fixed area", at),
        })?;
    let bytes = fixed_bytes(col, value)?;
    slot.copy_from_slice(&bytes);
    Ok(())
}

/// Fixed storage bytes (length always equals `fixed_len(col)`).
fn fixed_bytes(col: &JetColumn, value: &StorageValue) -> Result<Vec<u8>, StorageError> {
    let out = match (col.data_type, value) {
        (JetType::Byte, StorageValue::Int(i)) => vec![*i as u8],
        (JetType::Int, StorageValue::Int(i)) => (*i as i16).to_le_bytes().to_vec(),
        (JetType::LongInt, StorageValue::Int(i)) => (*i as i32).to_le_bytes().to_vec(),
        (JetType::Currency, StorageValue::Currency(c)) => c.to_le_bytes().to_vec(),
        (JetType::Float, StorageValue::Float(f)) => (*f as f32).to_le_bytes().to_vec(),
        (JetType::Double, StorageValue::Float(f)) => f.to_le_bytes().to_vec(),
        (JetType::DateTime, StorageValue::DateTime(d)) => d.to_le_bytes().to_vec(),
        (JetType::Guid, StorageValue::Guid(g)) => g.to_vec(),
        (JetType::Numeric, StorageValue::Decimal(d)) => encode_numeric(*d),
        (JetType::Binary, StorageValue::Binary(b)) if b.len() == fixed_len(col) => b.clone(),
        _ => return Err(type_error(col, value)),
    };
    Ok(out)
}

/// 96-bit scaled decimal in 17 bytes: sign byte (0x80 = positive) plus the
/// big-endian magnitude (inverse of `decode_numeric`).
fn encode_numeric(value: i128) -> Vec<u8> {
    let mut out = vec![0u8; 17];
    if value >= 0 {
        out[0] = 0x80;
    }
    let magnitude = value.unsigned_abs();
    for (i, byte) in magnitude.to_be_bytes().iter().rev().take(16).enumerate() {
        out[16 - i] = *byte;
    }
    out
}

/// Variable-column payload (slot bytes) for one value.
fn append_variable(
    var_data: &mut Vec<u8>,
    col: &JetColumn,
    value: &StorageValue,
) -> Result<(), StorageError> {
    let payload = match (col.data_type, value) {
        (JetType::Text, StorageValue::Text(s)) => encode_text(s, col),
        (JetType::Binary, StorageValue::Binary(b)) => b.clone(),
        (JetType::Memo, StorageValue::Text(s)) => {
            // Inline long-value form; LVAL chains are a later milestone.
            let body = encode_text(s, col);
            let mut out = (0x8000_0000u32 | body.len() as u32).to_le_bytes().to_vec();
            out.extend_from_slice(&body);
            out
        }
        (JetType::Ole, StorageValue::Binary(b)) => {
            let mut out = (0x8000_0000u32 | b.len() as u32).to_le_bytes().to_vec();
            out.extend_from_slice(b);
            out
        }
        _ => return Err(type_error(col, value)),
    };
    var_data.extend_from_slice(&payload);
    Ok(())
}

/// UCS-2LE text; compressed-flagged columns use the `FF FE` marker form
/// when every character fits a byte (access-compatible both ways).
fn encode_text(text: &str, col: &JetColumn) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if col.compressed_text && units.iter().all(|u| *u <= 0xFF) {
        let mut out = vec![0xFF, 0xFE];
        out.extend(units.iter().map(|u| *u as u8));
        return out;
    }
    let mut out = Vec::with_capacity(units.len() * 2);
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// Builds a table definition from ordered columns, deriving fixed offsets,
/// variable indices and storage lengths (the programmatic counterpart of
/// `JetColumn::new` fixtures). `used_pages`/`free_pages` are the raw TDEF
/// dwords (map type in the high byte, `(page << 8) | row` below).
pub fn build_table_def(
    table_type: u8,
    num_rows: u32,
    columns: &[JetColumn],
    used_pages: u32,
    free_pages: u32,
) -> JetTableDef {
    let mut fixed_offset = 0u16;
    let mut var_index = 0u16;
    let derived: Vec<JetColumn> = columns
        .iter()
        .map(|c| {
            let mut col = c.clone();
            if col.is_fixed() {
                if col.data_type != JetType::Bool {
                    col.fixed_offset = fixed_offset;
                    fixed_offset += col.len;
                }
            } else {
                col.fixed_offset = 0;
                col.var_index = Some(var_index);
                var_index += 1;
            }
            col
        })
        .collect();
    JetTableDef {
        table_type,
        num_rows,
        columns: derived,
        used_pages,
        free_pages,
    }
}

/// Encodes the TDEF block (inverse of `JetTableDef::decode`'s block parse):
/// 55-byte fixed header, `num_real_idx` × 12 (zero — no indexes yet), 25-byte
/// column descriptors in column-id order, UCS-2 names, per-variable usage-map
/// pointers terminated by `0xFFFF`.
pub fn encode_tdef(def: &JetTableDef) -> Vec<u8> {
    let columns = &def.columns;
    let num_var = columns.iter().filter(|c| !c.is_fixed()).count();
    let mut block = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&0u32.to_le_bytes()); // unknown
    block.extend_from_slice(&def.num_rows.to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x01);
    block.extend_from_slice(&[0u8; 3]);
    block.extend_from_slice(&0u32.to_le_bytes()); // complex autonumber
    block.extend_from_slice(&[0u8; 8]);
    block.push(def.table_type);
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes()); // max_cols
    block.extend_from_slice(&(num_var as u16).to_le_bytes());
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&def.used_pages.to_le_bytes());
    block.extend_from_slice(&def.free_pages.to_le_bytes());
    for col in columns {
        block.push(col.data_type as u8);
        block.extend_from_slice(&0u32.to_le_bytes()); // unknown
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&col.var_index.unwrap_or(0).to_le_bytes());
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&(u16::from(col.scale)).to_le_bytes()); // misc
        block.extend_from_slice(&0u16.to_le_bytes()); // misc_ext
        block.push(col.flags);
        block.push(u8::from(col.compressed_text)); // misc_flags
        block.extend_from_slice(&0u32.to_le_bytes()); // unknown
        block.extend_from_slice(&col.fixed_offset.to_le_bytes());
        block.extend_from_slice(&col.len.to_le_bytes());
    }
    for col in columns {
        let units: Vec<u16> = col.name.encode_utf16().collect();
        block.extend_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        for unit in units {
            block.extend_from_slice(&unit.to_le_bytes());
        }
    }
    block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // usage-pointer terminator
    let len = block.len() as u32;
    block[..4].copy_from_slice(&len.to_le_bytes());
    block
}

/// Packs a map type byte and a row pointer into a TDEF map dword.
pub fn pack_map_dword(kind: u8, page: u32, row: u8) -> u32 {
    (u32::from(kind) << 24) | crate::jet::usage_map::MapRef { page, row }.pack()
}

/// Convenience: nullable flags from the domain (kept symmetric with the
/// reader's mapping; unused by the writer core itself).
pub fn nullable_flag(nullable: bool) -> u8 {
    if nullable {
        flags::NULLABLE
    } else {
        flags::FIXED
    }
}
