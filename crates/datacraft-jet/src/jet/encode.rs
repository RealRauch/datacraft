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
use crate::jet::tdef::{JetColumn, JetTableDef, JetType};
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
    if num_cols > u16::MAX as usize {
        return Err(StorageError::Unsupported {
            reason: format!("a Jet row addresses at most 65535 columns, found {num_cols}"),
        });
    }
    let max_id = def
        .columns
        .iter()
        .map(|column| column.column_id as usize)
        .max()
        .unwrap_or(0);
    // The null mask follows the column-id space, not the descriptor count
    // (hostile files can declare sparse or oversized ids).
    if max_id / 8 >= num_cols.div_ceil(8) && num_cols > 0 {
        return Err(StorageError::Corrupt {
            context: "row encode".into(),
            detail: format!("column id {max_id} exceeds the null mask of {num_cols} columns"),
        });
    }
    let fixed_size = def
        .columns
        .iter()
        .filter(|c| c.is_fixed() && c.data_type != JetType::Bool)
        .map(|c| c.fixed_offset as usize + fixed_len(c))
        .max()
        .unwrap_or(0);

    let mut fixed = vec![0u8; fixed_size];
    let mut var_data: Vec<u8> = Vec::new();
    let mut mask = vec![0u8; num_cols.div_ceil(8)];

    // Every variable column gets a slot (NULLs take empty slots) — real
    // rows carry var_len for the full variable-column count, and the
    // decoder indexes slots by the column's var_index.
    let mut var_slots: Vec<(u16, usize)> = Vec::new();
    let mut sequential = 0u16;
    for (col, value) in def.columns.iter().zip(values) {
        let id = col.column_id as usize;
        if col.data_type == JetType::Bool {
            // The boolean value IS the mask bit — NULL is not representable
            // (M24 boundary, documented in FORMAT-JET "Domain mapping").
            match value {
                StorageValue::Bit(bit) => {
                    if *bit {
                        mask[id / 8] |= 1 << (id % 8);
                    }
                }
                StorageValue::Null => {
                    return Err(StorageError::Unsupported {
                        reason: format!(
                            "column {} ({}) is a Jet Yes/No column: Jet stores one bit and \
                             cannot represent NULL (a decoded NULL reads false) — write \
                             true/false or declare the column non-tri-state",
                            col.name, col.column_id
                        ),
                    });
                }
                other => return Err(type_error(col, other)),
            }
            continue;
        }
        if col.is_fixed() {
            if !matches!(value, StorageValue::Null) {
                mask[id / 8] |= 1 << (id % 8);
                write_fixed(&mut fixed, col, value)?;
            }
        } else {
            let start = var_data.len();
            if !matches!(value, StorageValue::Null) {
                mask[id / 8] |= 1 << (id % 8);
                append_variable(&mut var_data, col, value)?;
            }
            let index = col.var_index.unwrap_or(sequential);
            var_slots.push((index, start));
            sequential += 1;
        }
    }

    let eod = 2 + fixed_size + var_data.len();
    if eod > u16::MAX as usize {
        return Err(StorageError::Unsupported {
            reason: format!(
                "the encoded row needs {eod} bytes, beyond the 65535-byte Jet row limit"
            ),
        });
    }
    let mut row = Vec::with_capacity(eod + 2 + var_slots.len() * 2 + 2 + mask.len());
    row.extend_from_slice(&(num_cols as u16).to_le_bytes());
    row.extend_from_slice(&fixed);
    row.extend_from_slice(&var_data);
    row.extend_from_slice(&(eod as u16).to_le_bytes());
    // var_table is stored in reverse var order (entry 0 = the LAST
    // variable column), so write starts descending by var_index.
    var_slots.sort_by_key(|(index, _)| *index);
    for (_, start) in var_slots.iter().rev() {
        let at = start + 2 + fixed_size;
        row.extend_from_slice(&(at as u16).to_le_bytes());
    }
    row.extend_from_slice(&(var_slots.len() as u16).to_le_bytes());
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
    let bytes = scalar_bytes(col, value)?;
    slot.copy_from_slice(&bytes);
    Ok(())
}

/// Scalar storage bytes, identical in fixed and variable slots (real tables
/// store e.g. LongInt in variable slots — fixture nav-pane tables).
fn scalar_bytes(col: &JetColumn, value: &StorageValue) -> Result<Vec<u8>, StorageError> {
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
        (JetType::LongBinary, StorageValue::Binary(b)) if b.len() == fixed_len(col) => b.clone(),
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

/// Variable-column payload (slot bytes) for one value: long-value/text
/// forms plus any scalar type (variable LongInt columns are real).
fn append_variable(
    var_data: &mut Vec<u8>,
    col: &JetColumn,
    value: &StorageValue,
) -> Result<(), StorageError> {
    let payload = match (col.data_type, value) {
        (JetType::Text, StorageValue::Text(s)) => encode_text(s, col),
        (JetType::Binary, StorageValue::Binary(b)) => b.clone(),
        (JetType::LongBinary, StorageValue::Binary(b)) => b.clone(),
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
        // Scalars can live in variable slots; mismatches surface from
        // scalar_bytes as a precise type error.
        _ => scalar_bytes(col, value)?,
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
    let (index_section, index_count, real_index_count) = crate::jet::index::build_section(&derived);
    JetTableDef {
        table_type,
        num_rows,
        autonumber: if derived
            .iter()
            .any(|column| column.flags & crate::jet::tdef::flags::AUTONUMBER != 0)
        {
            1
        } else {
            0
        },
        columns: derived,
        used_pages,
        free_pages,
        indexes: crate::jet::index::decode_section(
            &index_section,
            index_count as usize,
            real_index_count as usize,
        )
        .map(|(indexes, _)| indexes)
        .unwrap_or_default(),
        index_section,
        index_count,
        real_index_count,
    }
}

/// Encodes the TDEF block (inverse of `JetTableDef::decode`'s block parse):
/// 55-byte fixed header, `num_real_idx` × 12 zero statistics descriptors,
/// 25-byte column descriptors in column-id order, UCS-2 names, the raw index
/// section, per-variable usage-map pointers terminated by `0xFFFF`.
///
/// Field constants observed in every real writer's TDEF (fresh-DAO reference
/// and the Access-2000 fixture alike, step 49.2): the head's second dword
/// and each descriptor's second dword carry the `0x659` constant, and the
/// per-column misc slot carries the `0x0409` collation (`0x0400 | scale`
/// for Numeric — modelled on `JetColumn::misc`). Written for byte parity;
/// the engine's open path may compare them.
pub fn encode_tdef(def: &JetTableDef) -> Vec<u8> {
    const TDEF_CONSTANT: u32 = 0x659;
    let columns = &def.columns;
    let num_var = columns.iter().filter(|c| !c.is_fixed()).count();
    let mut block = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&TDEF_CONSTANT.to_le_bytes()); // observed constant
    block.extend_from_slice(&def.num_rows.to_le_bytes());
    block.extend_from_slice(&def.autonumber.to_le_bytes());
    // The flag marks an AutoNumber column's presence (engines write 0 when
    // the table has none — observed 2026-11-07).
    block.push(u8::from(columns.iter().any(|column| {
        column.flags & crate::jet::tdef::flags::AUTONUMBER != 0
    })));
    block.extend_from_slice(&[0u8; 3]);
    block.extend_from_slice(&0u32.to_le_bytes()); // complex autonumber
    block.extend_from_slice(&[0u8; 8]);
    block.push(def.table_type);
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes()); // max_cols
    block.extend_from_slice(&(num_var as u16).to_le_bytes());
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&def.index_count.to_le_bytes()); // num_idx
    block.extend_from_slice(&def.real_index_count.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&def.used_pages.to_le_bytes());
    block.extend_from_slice(&def.free_pages.to_le_bytes());
    // Per-real-index row statistics: the observed middle dword carries the
    // table's row count (loose hints, not used for reads).
    for _ in 0..def.real_index_count {
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&def.num_rows.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
    }
    for col in columns {
        block.push(col.data_type as u8);
        block.extend_from_slice(&TDEF_CONSTANT.to_le_bytes()); // observed constant
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&col.var_index.unwrap_or(0).to_le_bytes());
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&col.misc.to_le_bytes()); // misc
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
    block.extend_from_slice(&def.index_section);
    block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // usage-pointer terminator
    // Fixed eight-byte trailer: every engine-written TDEF ends with it, and
    // the engine refuses a definition without it (DAO 3049; acceptance
    // finding 2026-11-07). `tdef_len` includes the trailer.
    block.extend_from_slice(&[0u8; 8]);
    let len = block.len() as u32;
    block[..4].copy_from_slice(&len.to_le_bytes());
    block
}

/// Packs a map type byte and a row pointer into a TDEF map dword.
pub fn pack_map_dword(kind: u8, page: u32, row: u8) -> u32 {
    (u32::from(kind) << 24) | crate::jet::usage_map::MapRef { page, row }.pack()
}
