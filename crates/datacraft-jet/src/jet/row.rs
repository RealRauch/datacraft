//! Jet4 row decoder (steps 2.3/2.4, item H6).
//!
//! Reference: `docs/FORMAT-JET.md` ("Rows", "Text compression", "Memo/OLE").
//! A row is decoded against its table's [`JetTableDef`]; the null mask drives
//! NULL detection (and boolean values); memo/OLE values resolve through LVAL
//! pages; text is UCS-2LE or compressed unicode.

use crate::error::StorageError;
use crate::jet::lval;
use crate::jet::tdef::{JetColumn, JetTableDef, JetType};
use crate::jet::text::{decode_compressed_text, decode_text};
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
    decode_row_with(row, def, file, page_size, false)
}

/// Decodes one Jet3 row: 1-byte counters and offsets, codepage (CP1252)
/// text, no compression. Rows whose data exceeds the 255-byte offset range
/// need the jump-table mechanism and answer a precise `Unsupported`
/// (docs/FORMAT-JET.md "Jet 3 (Access 97)").
pub fn decode_row_jet3(
    row: &[u8],
    def: &JetTableDef,
    file: &[u8],
    page_size: usize,
) -> Result<Vec<StorageValue>, StorageError> {
    decode_row_with(row, def, file, page_size, true)
}

fn decode_row_with(
    row: &[u8],
    def: &JetTableDef,
    file: &[u8],
    page_size: usize,
    jet3: bool,
) -> Result<Vec<StorageValue>, StorageError> {
    let layout = RowLayout::parse(row, jet3)?;
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
            values.push(decode_fixed(row, col, jet3)?);
        } else {
            values.push(decode_variable(row, &layout, col, file, page_size, jet3)?);
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
    /// Jump table between the var table and `var_len` (Jet3 only; empty on
    /// Jet4 and on rows below the 256-byte offset range).
    jump_at: usize,
    jump_len: usize,
    eod: usize,
    jet3: bool,
}

impl RowLayout {
    fn parse(row: &[u8], jet3: bool) -> Result<Self, StorageError> {
        if jet3 {
            return Self::parse_jet3(row);
        }
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
            jump_at: 0,
            jump_len: 0,
            eod,
            jet3,
        })
    }

    /// Jet3 tail: `[eod u8][var_table u8 × n][jump_table][var_len u8][mask]`.
    /// The jump table appears when the row crosses 256-byte boundaries; its
    /// entries name the first variable column of each 256-byte segment (a
    /// `0xFF` entry is the documented dummy). The number of entries follows
    /// the row size; the closure check picks the documented candidate and
    /// falls back to the small candidate range for hostile files.
    fn parse_jet3(row: &[u8]) -> Result<Self, StorageError> {
        if row.is_empty() {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: "row shorter than num_cols field (0 bytes)".into(),
            });
        }
        let num_cols = row[0] as usize;
        let mask_len = num_cols.div_ceil(8);
        if row.len() < 1 + mask_len + 2 {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: "row too short for null mask".into(),
            });
        }
        let mask_at = row.len() - mask_len;
        let var_len = row[mask_at - 1] as usize;
        // The documented count follows the row size (`row_len / 256`); the
        // closure check is the authority, with a small fallback range for
        // files that disagree.
        let preferred = row.len() / 256;
        let mut candidates = vec![preferred];
        for candidate in 0..=preferred + 1 {
            if candidate != preferred {
                candidates.push(candidate);
            }
        }
        let mut chosen = None;
        for jump_len in candidates {
            let Some(jump_at) = (mask_at - 1).checked_sub(jump_len) else {
                continue;
            };
            let Some(table_at) = jump_at.checked_sub(var_len) else {
                continue;
            };
            if table_at < 2 {
                continue;
            }
            let raw_eod = row[table_at - 1] as usize;
            if !row[jump_at..jump_at + jump_len]
                .iter()
                .all(|entry| *entry == 0xFF || (*entry as usize) <= var_len)
            {
                continue;
            }
            // The raw eod is a byte; every crossed 256-byte boundary (a jump
            // entry at or below the eod's column index) adds one segment.
            let segments = row[jump_at..jump_at + jump_len]
                .iter()
                .filter(|entry| **entry != 0xFF && **entry as usize <= var_len)
                .count();
            let eod = raw_eod + segments * 256;
            if eod + 1 + var_len + jump_len + 1 + mask_len != row.len() {
                continue;
            }
            chosen = Some((table_at, jump_at, jump_len, eod));
            break;
        }
        let Some((table_at, jump_at, jump_len, eod)) = chosen else {
            return Err(StorageError::Corrupt {
                context: "row".into(),
                detail: format!(
                    "Jet3 row tail does not close ({} bytes, {} variable columns)",
                    row.len(),
                    var_len
                ),
            });
        };
        Ok(Self {
            num_cols,
            mask_at,
            var_len,
            table_at,
            jump_at,
            jump_len,
            eod,
            jet3: true,
        })
    }

    /// Null-mask bit for column id `index` — bits follow the column-id space,
    /// LSB first (`0` = NULL, except booleans whose value is the bit).
    fn mask_bit(&self, row: &[u8], index: usize) -> bool {
        row[self.mask_at + index / 8] >> (index % 8) & 1 == 1
    }

    /// Start offset of the row's variable column `j` (var_table is stored in
    /// reverse order: entry 0 belongs to the last variable column). Entries
    /// are 2-byte on Jet4 and 1-byte on Jet3. Always in bounds: the largest
    /// entry address is `mask_at - 4` (Jet4) / `mask_at - 2` (Jet3) by the
    /// parse invariants, so no bounds check is needed here.
    fn var_start(&self, row: &[u8], j: usize) -> usize {
        if self.jet3 {
            let stored = row[self.table_at + (self.var_len - 1 - j)] as usize;
            if self.jump_len == 0 {
                return stored;
            }
            // Each jump entry at or below this variable column marks a
            // crossed 256-byte boundary (the 0xFF dummy is ignored).
            let segments = row[self.jump_at..self.jump_at + self.jump_len]
                .iter()
                .filter(|entry| **entry != 0xFF && **entry as usize <= j)
                .count();
            return stored + segments * 256;
        }
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

fn decode_fixed(row: &[u8], col: &JetColumn, jet3: bool) -> Result<StorageValue, StorageError> {
    // Jet4 rows carry a 2-byte num_cols field, Jet3 rows a 1-byte one —
    // the fixed area starts right behind it.
    let start = if jet3 { 1 } else { 2 } + col.fixed_offset as usize;
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
    decode_bytes(bytes, col, jet3)
}

/// Storage length of a fixed column; numerics always occupy 17 bytes
/// regardless of the stored length (booleans never reach byte decoding —
/// their value lives in the null mask).
pub(crate) fn fixed_len(col: &JetColumn) -> usize {
    match col.data_type {
        JetType::Numeric => 17,
        _ => col.len as usize,
    }
}

/// Whether any memo/OLE column of `row` points at an external long-value
/// storage form (single LVAL record or chain) instead of the inline form
/// the writer emits. Updating such a row would orphan the LVAL pages, so
/// the writer refuses it precisely.
pub(crate) fn has_external_long_value(row: &[u8], def: &JetTableDef) -> Result<bool, StorageError> {
    let layout = RowLayout::parse(row, false)?;
    for col in &def.columns {
        if col.is_fixed() || !matches!(col.data_type, JetType::Memo | JetType::Ole) {
            continue;
        }
        let index = usize::from(col.var_index.unwrap_or(0));
        if index >= layout.var_len {
            continue;
        }
        let (start, end) = (layout.var_start(row, index), layout.var_end(row, index));
        if end <= start {
            continue; // NULL (empty slot)
        }
        if end - start < 4 {
            return Ok(true); // malformed payload: never rewrite it blindly
        }
        let flags =
            u32::from_le_bytes([row[start], row[start + 1], row[start + 2], row[start + 3]])
                & 0xC000_0000;
        if flags != 0x8000_0000 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn decode_variable(
    row: &[u8],
    layout: &RowLayout,
    col: &JetColumn,
    file: &[u8],
    page_size: usize,
    jet3: bool,
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
        JetType::Memo | JetType::Ole => decode_long_value(bytes, col, file, page_size, jet3),
        _ => decode_bytes(bytes, col, jet3),
    }
}

/// 12-byte (single/chained) or 4-byte (inline) memo header, per
/// `docs/FORMAT-JET.md`.
fn decode_long_value(
    bytes: &[u8],
    col: &JetColumn,
    file: &[u8],
    page_size: usize,
    jet3: bool,
) -> Result<StorageValue, StorageError> {
    if bytes.len() < 4 {
        return Err(StorageError::Corrupt {
            context: "row".into(),
            detail: format!("memo column {} shorter than its header", col.name),
        });
    }
    let length_and_flags = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let data: Vec<u8> = if length_and_flags & 0xC000_0000 == 0x8000_0000 {
        // Inline: [dword len|0x80000000][payload]. Access 2000+ prepends
        // an 8-byte zero header that the length does not count (fixture
        // evidence, queryTestV2000.mdb); our own writer omits it. The
        // slot length decides which form is present.
        let len = (length_and_flags & 0x3FFF_FFFF) as usize;
        let start = if bytes.len() >= 12 + len && bytes[4..12].iter().all(|b| *b == 0) {
            12
        } else {
            4
        };
        bytes
            .get(start..start + len)
            .ok_or_else(|| StorageError::Corrupt {
                context: "row".into(),
                detail: format!(
                    "inline memo {} declares {len} bytes but holds {}",
                    col.name,
                    bytes.len() - start
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
        lval::resolve(file, page_size, length_and_flags, pointer, jet3)?
    };
    Ok(match col.data_type {
        // Memos are always compressed unicode on Jet4 (FORMAT-JET.md,
        // "Memo"); plain text columns follow the column's flag.
        JetType::Memo if !jet3 => StorageValue::Text(decode_compressed_text(&data)),
        JetType::Memo => StorageValue::Text(crate::jet::jet3::cp1252(&data)),
        _ => StorageValue::Binary(data),
    })
}

fn decode_bytes(bytes: &[u8], col: &JetColumn, jet3: bool) -> Result<StorageValue, StorageError> {
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

#[cfg(test)]
mod tests;
