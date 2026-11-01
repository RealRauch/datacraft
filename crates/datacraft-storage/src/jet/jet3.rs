//! Jet 3 (Access 97) read path (step 10.6, item L1).
//!
//! Facts per `docs/FORMAT-JET.md` "Jet 3 (Access 97)" (public-doc source:
//! MDB Tools HACKING). The TDEF uses a 35-byte block header with 18-byte
//! descriptors and CP1252 names; rows use 1-byte counters/offsets; text is
//! codepage-encoded (CP1252 is the documented writer default). Rows whose
//! data exceeds 255 bytes need the jump-table mechanism and answer a
//! precise `Unsupported` (documented L1 remainder).

use crate::error::StorageError;
use crate::jet::tdef::{JetTableDef, JetType};

impl JetTableDef {
    /// Decodes a Jet3 TDEF (docs/FORMAT-JET.md "Jet 3 (Access 97)"): a
    /// 35-byte block header, 18-byte column descriptors, 1-byte-length
    /// codepage names. Jet3 has no compressed text and carries the code
    /// page in the text columns' `misc` field; names and cell text decode
    /// as CP1252 (the documented writer-machine default).
    pub fn decode_tdef(file: &[u8], tdef_page: u32) -> Result<Self, StorageError> {
        let header = crate::jet::header::FileHeader::decode(
            file.get(..0x18).unwrap_or(file),
            file.len() as u64,
        )?;
        // Chained TDEFs concatenate their post-header payloads; the first
        // page's `tdef_len` is the total block length (continuation pages
        // carry arbitrary bytes in that field). Verified on the pinned
        // fixture: page 34 announces 2815 bytes and chains to page 44.
        let mut buffer: Vec<u8> = Vec::new();
        let mut page_no = tdef_page;
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(page_no) {
                return Err(StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("Jet3 TDEF page chain revisits page {page_no}"),
                });
            }
            let page = header.page_slice(file, page_no)?;
            if crate::jet::page::page_type(page) != crate::jet::page::PAGE_TYPE_TDEF {
                return Err(StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("page {page_no} in a Jet3 TDEF chain is not a TDEF page"),
                });
            }
            buffer.extend_from_slice(&page[8..]);
            let next_pg = u32::from_le_bytes([page[4], page[5], page[6], page[7]]);
            if next_pg == 0 {
                break;
            }
            page_no = next_pg;
        }
        let tdef_len = le::u32(&buffer, 0, "tdef_len")? as usize;
        if tdef_len < 35 || tdef_len > buffer.len() {
            return Err(StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("Jet3 tdef_len {tdef_len} out of bounds ({})", buffer.len()),
            });
        }
        let b = &buffer[..tdef_len];
        let num_rows = le::u32(b, 4, "num_rows")?;
        let table_type = *b.get(12).ok_or_else(|| StorageError::Corrupt {
            context: "table definition".into(),
            detail: "truncated before table_type".into(),
        })?;
        let _max_cols = le::u16(b, 13, "max_cols")?;
        let _num_var_cols = le::u16(b, 15, "num_var_cols")?;
        let num_cols = le::u16(b, 17, "num_cols")? as usize;
        let _num_idx = le::u32(b, 19, "num_idx")? as usize;
        let num_real_idx = le::u32(b, 23, "num_real_idx")? as usize;
        let used_pages = le::u32(b, 27, "used_pages")?;
        let free_pages = le::u32(b, 31, "free_pages")?;
        let mut at = 35 + num_real_idx * 8; // real-index header stubs
        let mut columns = Vec::with_capacity(num_cols);
        for _ in 0..num_cols {
            let raw_type = *b.get(at).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated column descriptor at {at}"),
            })?;
            let data_type = JetType::from_byte(raw_type)?;
            let column_id = le::u16(b, at + 1, "col_num")?;
            let var_index = le::u16(b, at + 3, "offset_V")?;
            let misc = le::u16(b, at + 9, "misc")?;
            let flag_bits = *b.get(at + 13).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated column flags at {}", at + 13),
            })?;
            let fixed_offset = le::u16(b, at + 14, "fixed_offset")?;
            let len = le::u16(b, at + 16, "col_len")?;
            columns.push(crate::jet::tdef::JetColumn {
                name: String::new(), // filled below
                data_type,
                flags: flag_bits,
                column_id,
                var_index: (flag_bits & crate::jet::tdef::flags::FIXED == 0).then_some(var_index),
                fixed_offset,
                len,
                scale: (misc & 0xFF) as u8,
                compressed_text: false, // Jet3 has no unicode compression
            });
            at += 18;
        }
        for col in &mut columns {
            let name_len = *b.get(at).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated name length at {at}"),
            })? as usize;
            let bytes = b
                .get(at + 1..at + 1 + name_len)
                .ok_or_else(|| StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("truncated column name at {at}"),
                })?;
            col.name = decode_cp1252(bytes);
            at += 1 + name_len;
        }
        columns.sort_by_key(|c| c.column_id);
        Ok(Self {
            table_type,
            num_rows,
            columns,
            used_pages,
            free_pages,
        })
    }
}

/// CP1252 → text (the documented Jet3 writer default; 0x80–0x9F map
/// through the Windows-1252 table, everything else is identity).
fn decode_cp1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    bytes
        .iter()
        .map(|&b| {
            if (0x80..0xA0).contains(&b) {
                HIGH[b as usize - 0x80]
            } else {
                b as char
            }
        })
        .collect()
}

/// CP1252 → String (exposed for the row decoder's text columns).
pub(crate) fn cp1252(bytes: &[u8]) -> String {
    decode_cp1252(bytes)
}

/// Little-endian readers with precise bounds errors (same shape as the
/// Jet4 decoder's `le` module).
pub(crate) mod le {
    use super::StorageError;

    pub fn u16(data: &[u8], at: usize, what: &str) -> Result<u16, StorageError> {
        data.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated {what} at offset {at}"),
            })
    }

    pub fn u32(data: &[u8], at: usize, what: &str) -> Result<u32, StorageError> {
        data.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated {what} at offset {at}"),
            })
    }
}
