//! Jet4 row decoder (steps 2.3/2.4, item H6).
//!
//! Reference: `docs/FORMAT-JET.md` ("Rows", "Text compression", "Memo/OLE").
//! A row is decoded against its table's [`JetTableDef`]; the null mask drives
//! NULL detection (and boolean values); memo/OLE values resolve through LVAL
//! pages; text is UCS-2LE or compressed unicode.

use crate::error::StorageError;
use crate::jet::lval;
use crate::jet::tdef::{JetColumn, JetTableDef, JetType};
use crate::types::StorageValue;

/// Decodes one raw row against the table definition.
///
/// `row` is the record slice from the data page (starting with `num_cols`);
/// `file`/`page_size` provide LVAL page access for memo/OLE values.
///
/// Null-mask bits follow the **column id**, not the descriptor position
/// (verified against both pinned writers of `fixtures/testV2000.mdb` —
/// real Access and jackcess; `docs/FORMAT-JET.md` "Rows").
pub fn decode_row(
    row: &[u8],
    def: &JetTableDef,
    file: &[u8],
    page_size: usize,
) -> Result<Vec<StorageValue>, StorageError> {
    let layout = RowLayout::parse(row)?;
    let mut values = Vec::with_capacity(def.columns.len());
    for col in &def.columns {
        if col.column_id as usize >= layout.num_cols {
            // Column added after this row was written (rows store a prefix
            // of the column-id space).
            values.push(StorageValue::Null);
            continue;
        }
        let mask_bit = layout.mask_bit(row, col.column_id as usize);
        if col.data_type == JetType::Bool {
            // Booleans store their value in the null mask itself.
            values.push(StorageValue::Bit(mask_bit));
            continue;
        }
        if !mask_bit {
            values.push(StorageValue::Null);
            continue;
        }
        if col.is_fixed() {
            values.push(decode_fixed(row, col)?);
        } else {
            values.push(decode_variable(row, &layout, col, file, page_size)?);
        }
    }
    Ok(values)
}

/// The end-anchored variable layout of one row, parsed once. Tail field
/// order as verified against both pinned writers of the oracle fixture
/// (`docs/FORMAT-JET.md` "Rows"): `[eod][var_table × n, reverse][var_len]
/// [null mask]`.
struct RowLayout {
    num_cols: usize,
    mask_at: usize,
    var_len: usize,
    table_at: usize,
    eod: usize,
}

impl RowLayout {
    fn parse(row: &[u8]) -> Result<Self, StorageError> {
        if row.len() < 2 {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: format!("row shorter than num_cols field ({} bytes)", row.len()),
            });
        }
        let num_cols = u16::from_le_bytes([row[0], row[1]]) as usize;
        let mask_len = num_cols.div_ceil(8);
        if row.len() < 2 + mask_len + 2 {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: "row too short for null mask".into(),
            });
        }
        let mask_at = row.len() - mask_len;
        let var_len = u16::from_le_bytes([row[mask_at - 2], row[mask_at - 1]]) as usize;
        let table_at = (mask_at - 2).checked_sub(var_len * 2);
        let table_at = match table_at {
            Some(at) if at >= 2 => at,
            _ => {
                return Err(StorageError::Corrupt {
                    context: "row".into(),
                    detail: "row too short for variable column table".into(),
                });
            }
        };
        let eod = u16::from_le_bytes([row[table_at - 2], row[table_at - 1]]) as usize;
        if eod > row.len() {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: format!("eod {eod} lies beyond the row ({} bytes)", row.len()),
            });
        }
        Ok(Self {
            num_cols,
            mask_at,
            var_len,
            table_at,
            eod,
        })
    }

    /// Null-mask bit for column id `index` — bits follow the column-id space,
    /// LSB first (`0` = NULL, except booleans whose value is the bit).
    fn mask_bit(&self, row: &[u8], index: usize) -> bool {
        row[self.mask_at + index / 8] >> (index % 8) & 1 == 1
    }

    /// Start offset of the row's variable column `j` (var_table is stored in
    /// reverse order: entry 0 belongs to the last variable column). Always in
    /// bounds: the largest entry address is `mask_at - 4` by the parse
    /// invariants, so no bounds check is needed here.
    fn var_start(&self, row: &[u8], j: usize) -> usize {
        let at = self.table_at + (self.var_len - 1 - j) * 2;
        u16::from_le_bytes([row[at], row[at + 1]]) as usize
    }

    /// End offset of variable column `j`: start of `j+1`, `eod` for the last.
    fn var_end(&self, row: &[u8], j: usize) -> usize {
        if j + 1 >= self.var_len {
            self.eod
        } else {
            self.var_start(row, j + 1)
        }
    }
}

fn decode_fixed(row: &[u8], col: &JetColumn) -> Result<StorageValue, StorageError> {
    let start = 2 + col.fixed_offset as usize;
    let len = fixed_len(col);
    let bytes = row
        .get(start..start + len)
        .ok_or_else(|| StorageError::Corrupt {
            context: "row".into(),
            detail: format!(
                "fixed column {} ({}) needs bytes {start}..{}, row holds {}",
                col.column_id,
                col.name,
                start + len,
                row.len()
            ),
        })?;
    decode_bytes(bytes, col)
}

/// Storage length of a fixed column; numerics always occupy 17 bytes
/// regardless of the stored length (booleans never reach byte decoding —
/// their value lives in the null mask).
fn fixed_len(col: &JetColumn) -> usize {
    match col.data_type {
        JetType::Numeric => 17,
        _ => col.len as usize,
    }
}

fn decode_variable(
    row: &[u8],
    layout: &RowLayout,
    col: &JetColumn,
    file: &[u8],
    page_size: usize,
) -> Result<StorageValue, StorageError> {
    let j = usize::from(col.var_index.unwrap_or(0));
    if j >= layout.var_len {
        // Variable column added after this row was written.
        return Ok(StorageValue::Null);
    }
    let start = layout.var_start(row, j);
    let end = layout.var_end(row, j);
    if start == end {
        // Empty variable slot: NULL for long-value columns, empty string for
        // text (mdb-compat — real rows point empty slots at the eod).
        return Ok(match col.data_type {
            JetType::Text => StorageValue::Text(String::new()),
            _ => StorageValue::Null,
        });
    }
    if start > end || end > row.len() {
        return Err(StorageError::Corrupt {
            context: "row".into(),
            detail: format!(
                "variable column {} ({}) has invalid range {start}..{end}",
                col.column_id, col.name
            ),
        });
    }
    let bytes = &row[start..end];
    match col.data_type {
        JetType::Memo | JetType::Ole => decode_long_value(bytes, col, file, page_size),
        _ => decode_bytes(bytes, col),
    }
}

/// 12-byte (single/chained) or 4-byte (inline) memo header, per
/// `docs/FORMAT-JET.md`.
fn decode_long_value(
    bytes: &[u8],
    col: &JetColumn,
    file: &[u8],
    page_size: usize,
) -> Result<StorageValue, StorageError> {
    if bytes.len() < 4 {
        return Err(StorageError::Corrupt {
            context: "row".into(),
            detail: format!("memo column {} shorter than its header", col.name),
        });
    }
    let length_and_flags = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let data: Vec<u8> = if length_and_flags & 0xC000_0000 == 0x8000_0000 {
        // Inline: payload follows the length dword directly.
        let len = (length_and_flags & 0x3FFF_FFFF) as usize;
        bytes
            .get(4..4 + len)
            .ok_or_else(|| StorageError::Corrupt {
                context: "row".into(),
                detail: format!(
                    "inline memo {} declares {len} bytes but holds {}",
                    col.name,
                    bytes.len() - 4
                ),
            })?
            .to_vec()
    } else {
        if bytes.len() < 12 {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: format!("memo column {} shorter than its 12-byte header", col.name),
            });
        }
        let pointer = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        lval::resolve(file, page_size, length_and_flags, pointer)?
    };
    Ok(match col.data_type {
        JetType::Memo => StorageValue::Text(decode_text_bytes(&data, col)),
        _ => StorageValue::Binary(data),
    })
}

fn decode_bytes(bytes: &[u8], col: &JetColumn) -> Result<StorageValue, StorageError> {
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
            StorageValue::Float(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64)
        }
        JetType::Double => StorageValue::Float(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        JetType::DateTime => StorageValue::DateTime(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        JetType::Binary | JetType::Ole | JetType::Complex | JetType::Memo => {
            StorageValue::Binary(bytes.to_vec())
        }
        JetType::Text => StorageValue::Text(decode_text_bytes(bytes, col)),
        JetType::Guid => {
            let mut guid = [0u8; 16];
            guid.copy_from_slice(bytes);
            StorageValue::Guid(guid)
        }
        JetType::Numeric => decode_numeric(bytes),
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

/// UCS-2LE or compressed-unicode text (see `docs/FORMAT-JET.md`).
fn decode_text_bytes(bytes: &[u8], col: &JetColumn) -> String {
    if col.compressed_text {
        decode_compressed_text(bytes)
    } else {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .collect();
        String::from_utf16_lossy(&units)
    }
}

/// Compressed unicode: optional `FF FE` marker, then single-byte characters
/// until a `0x00` switches to UCS-2 pairs; a pair with high byte `0` reads
/// back as its low byte (identical to the compressed reading).
fn decode_compressed_text(bytes: &[u8]) -> String {
    let body = if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    };
    let mut units: Vec<u16> = Vec::new();
    let mut compressed = bytes.starts_with(&[0xFF, 0xFE]);
    let mut at = 0usize;
    while at < body.len() {
        if compressed {
            let b = body[at];
            at += 1;
            if b == 0x00 {
                compressed = false;
            } else {
                units.push(u16::from(b));
            }
        } else if at + 2 <= body.len() {
            let lo = body[at];
            let hi = body[at + 1];
            at += 2;
            units.push((u16::from(hi) << 8) | u16::from(lo));
        } else {
            break;
        }
    }
    String::from_utf16_lossy(&units)
}
