//! Table definition (TDEF) decoder (step 2.3, item H6).
//!
//! Layout reference: `docs/FORMAT-JET.md` ("Table definition pages"). The
//! block is read from offset 8 of the TDEF page (after the 8-byte page
//! header). Multi-page TDEFs and index blocks are parsed (currently only to
//! skip them); their semantics arrive with the oracle step.

use crate::error::StorageError;
use crate::types::StorageDataType;

/// Logical Jet/ACE column type (TDEF type byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JetType {
    /// Boolean (value lives in the row null mask).
    Bool = 0x01,
    /// 8-bit integer.
    Byte = 0x02,
    /// 16-bit integer.
    Int = 0x03,
    /// 32-bit integer.
    LongInt = 0x04,
    /// Currency (scaled 64-bit integer, 10⁻⁴).
    Currency = 0x05,
    /// 32-bit float.
    Float = 0x06,
    /// 64-bit float.
    Double = 0x07,
    /// Date/time (OLE automation f64).
    DateTime = 0x08,
    /// Binary (up to 255 bytes).
    Binary = 0x09,
    /// Text (up to 255 chars).
    Text = 0x0A,
    /// Long binary (OLE object).
    Ole = 0x0B,
    /// Long text (memo).
    Memo = 0x0C,
    /// GUID.
    Guid = 0x0F,
    /// 96-bit scaled decimal stored in 17 bytes.
    Numeric = 0x10,
    /// Complex (multi-value/attachment) column.
    Complex = 0x12,
}

impl JetType {
    /// Maps a TDEF type byte; unknown values are a precise error.
    pub fn from_byte(byte: u8) -> Result<Self, StorageError> {
        Ok(match byte {
            0x01 => Self::Bool,
            0x02 => Self::Byte,
            0x03 => Self::Int,
            0x04 => Self::LongInt,
            0x05 => Self::Currency,
            0x06 => Self::Float,
            0x07 => Self::Double,
            0x08 => Self::DateTime,
            0x09 => Self::Binary,
            0x0A => Self::Text,
            0x0B => Self::Ole,
            0x0C => Self::Memo,
            0x0F => Self::Guid,
            0x10 => Self::Numeric,
            0x12 => Self::Complex,
            other => {
                return Err(StorageError::Unsupported {
                    reason: format!("column type 0x{other:02x} not supported"),
                });
            }
        })
    }

    /// The physical storage type this maps onto at the `StorageEngine`
    /// boundary (no Jet type leaks across it).
    pub fn storage_type(self) -> StorageDataType {
        match self {
            Self::Bool => StorageDataType::Bit,
            Self::Byte => StorageDataType::Int1,
            Self::Int => StorageDataType::Int2,
            Self::LongInt => StorageDataType::Int4,
            Self::Currency => StorageDataType::Currency,
            Self::Float => StorageDataType::Float4,
            Self::Double => StorageDataType::Float8,
            Self::DateTime => StorageDataType::DateTime,
            Self::Binary | Self::Ole => StorageDataType::Binary,
            Self::Text => StorageDataType::Text,
            Self::Memo => StorageDataType::Memo,
            Self::Guid => StorageDataType::Guid,
            Self::Numeric => StorageDataType::Decimal,
            Self::Complex => StorageDataType::Complex,
        }
    }
}

/// Column flags (TDEF bitmask byte).
pub mod flags {
    /// Fixed-length column.
    pub const FIXED: u8 = 0x01;
    /// Nullable column.
    pub const NULLABLE: u8 = 0x02;
    /// AutoNumber column.
    pub const AUTONUMBER: u8 = 0x04;
}

/// One decoded column descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetColumn {
    /// Column name (UCS-2LE in the file).
    pub name: String,
    /// Logical Jet type.
    pub data_type: JetType,
    /// TDEF flag bits (`flags` module).
    pub flags: u8,
    /// Column id (stable across schema changes).
    pub column_id: u16,
    /// Index among the variable columns, if variable.
    pub var_index: Option<u16>,
    /// Offset of fixed columns within the row's fixed area (after `num_cols`).
    pub fixed_offset: u16,
    /// Storage length: exact for fixed, max for variable, 0 for memo/OLE.
    pub len: u16,
    /// Scale for `Numeric` columns (from the `misc` descriptor byte).
    pub scale: u8,
    /// True when the text column uses compressed unicode.
    pub compressed_text: bool,
}

impl JetColumn {
    /// Creates a descriptor (scale 0, uncompressed text) — test fixture helper
    /// and programmatic schema construction.
    pub fn new(
        name: &str,
        data_type: JetType,
        flags: u8,
        column_id: u16,
        fixed_offset: u16,
        len: u16,
    ) -> Self {
        Self {
            name: name.to_owned(),
            data_type,
            flags,
            column_id,
            var_index: None,
            fixed_offset,
            len,
            scale: 0,
            compressed_text: false,
        }
    }

    /// Whether the column is fixed-length (`0x01` flag).
    pub fn is_fixed(&self) -> bool {
        self.flags & flags::FIXED != 0
    }

    /// Whether the column may be NULL (`0x02` flag).
    pub fn is_nullable(&self) -> bool {
        self.flags & flags::NULLABLE != 0
    }
}

/// A decoded table definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetTableDef {
    /// `0x4E` user table, `0x53` system table.
    pub table_type: u8,
    /// Total row count of the table.
    pub num_rows: u32,
    /// Column descriptors in storage order.
    pub columns: Vec<JetColumn>,
    /// Raw `used_pages` dword: usage-map type byte plus the row pointer of
    /// the table's page-ownership map (writer, step 10.1).
    pub used_pages: u32,
    /// Raw `free_pages` dword (same packing, map of pages with free space).
    pub free_pages: u32,
}

impl JetTableDef {
    /// Maps the Jet columns onto the boundary schema types (no Jet type
    /// leaks across the `StorageEngine` boundary).
    pub fn schema_columns(&self) -> Vec<crate::types::ColumnSchema> {
        self.columns
            .iter()
            .map(|c| {
                crate::types::ColumnSchema::new(
                    c.name.clone(),
                    c.data_type.storage_type(),
                    c.is_nullable(),
                )
            })
            .collect()
    }
}

/// Little-endian u16/u32 readers with precise bounds errors.
mod le {
    use super::StorageError;

    pub fn u16(data: &[u8], at: usize, what: &str) -> Result<u16, StorageError> {
        data.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| truncated(what, at, 2, data.len()))
    }

    pub fn u32(data: &[u8], at: usize, what: &str) -> Result<u32, StorageError> {
        data.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| truncated(what, at, 4, data.len()))
    }

    fn truncated(what: &str, at: usize, need: usize, have: usize) -> StorageError {
        StorageError::Corrupt {
            context: "table definition".into(),
            detail: format!("truncated {what} at offset {at}: need {need} bytes, have {have}"),
        }
    }
}

impl JetTableDef {
    /// Decodes the TDEF of `tdef_page` from the raw file bytes.
    ///
    /// Index definitions are parsed past (their semantics are not needed for
    /// reads and land with the oracle step).
    pub fn decode(file: &[u8], tdef_page: u32) -> Result<Self, StorageError> {
        let header = crate::jet::header::FileHeader::decode(
            file.get(..0x18).unwrap_or(file),
            file.len() as u64,
        )?;
        // Multi-page TDEFs concatenate their post-header payloads via the
        // next-TDEF-page pointer at offset 4 (docs/FORMAT-JET.md). The
        // logical block is the concatenation; `tdef_len` (first field,
        // self-inclusive) still bounds the parse.
        let mut buffer: Vec<u8> = Vec::new();
        let mut page_no = tdef_page;
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(page_no) {
                return Err(StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("TDEF page chain revisits page {page_no}"),
                });
            }
            let page = header.page_slice(file, page_no)?;
            if crate::jet::page::page_type(page) != crate::jet::page::PAGE_TYPE_TDEF {
                return Err(StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("page {page_no} in a TDEF chain is not a TDEF page"),
                });
            }
            buffer.extend_from_slice(&page[8..]);
            let next_pg = u32::from_le_bytes([page[4], page[5], page[6], page[7]]);
            if next_pg == 0 {
                break;
            }
            page_no = next_pg;
        }
        // The block starts right after the 8-byte page header; its first
        // field announces its own (self-inclusive) length (docs/FORMAT-JET.md).
        let tdef_len = le::u32(&buffer, 0, "tdef_len")? as usize;
        if tdef_len > buffer.len() {
            return Err(StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!(
                    "tdef_len {tdef_len} exceeds the chained block size {}",
                    buffer.len()
                ),
            });
        }
        let b = &buffer[..tdef_len];

        let _unknown0 = le::u32(b, 4, "unknown")?;
        let num_rows = le::u32(b, 8, "num_rows")?;
        let _autonumber = le::u32(b, 12, "autonumber")?;
        // +16: autonum flag (0x01) + 3 unknown, +20: complex autonumber,
        // +24: 8 unknown, +32: table type.
        let table_type = *b.get(32).ok_or_else(|| StorageError::Corrupt {
            context: "table definition".into(),
            detail: "truncated before table_type".into(),
        })?;
        let max_cols = le::u16(b, 33, "max_cols")?;
        let num_var_cols = le::u16(b, 35, "num_var_cols")?;
        let num_cols = le::u16(b, 37, "num_cols")? as usize;
        let num_idx = le::u32(b, 39, "num_idx")? as usize;
        let num_real_idx = le::u32(b, 43, "num_real_idx")? as usize;
        let used_pages = le::u32(b, 47, "used_pages")?;
        let free_pages = le::u32(b, 51, "free_pages")?;
        let mut at = 55 + num_real_idx * 12;
        let mut columns = Vec::with_capacity(num_cols);
        for _ in 0..num_cols {
            let raw_type = *b.get(at).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated column descriptor at {at}"),
            })?;
            let data_type = JetType::from_byte(raw_type)?;
            let _unknown = le::u32(b, at + 1, "column unknown")?;
            let column_id = le::u16(b, at + 5, "column_id")?;
            let var_index = le::u16(b, at + 7, "var_index")?;
            let _col_num2 = le::u16(b, at + 9, "column_id(2)")?;
            let misc = le::u16(b, at + 11, "misc")?;
            let _misc_ext = le::u16(b, at + 13, "misc_ext")?;
            let flag_bits = *b.get(at + 15).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated column flags at {}", at + 15),
            })?;
            let misc_flags = *b.get(at + 16).ok_or_else(|| StorageError::Corrupt {
                context: "table definition".into(),
                detail: format!("truncated misc_flags at {}", at + 16),
            })?;
            let _unknown2 = le::u32(b, at + 17, "column unknown(2)")?;
            let fixed_offset = le::u16(b, at + 21, "fixed_offset")?;
            let len = le::u16(b, at + 23, "col_len")?;
            columns.push(JetColumn {
                name: String::new(), // filled below
                data_type,
                flags: flag_bits,
                column_id,
                var_index: (flag_bits & flags::FIXED == 0).then_some(var_index),
                fixed_offset,
                len,
                scale: (misc & 0xFF) as u8,
                compressed_text: misc_flags & 0x01 != 0,
            });
            at += 25;
        }
        for col in &mut columns {
            let name_len = le::u16(b, at, "name_len")? as usize;
            // The length field counts BYTES of the UCS-2 name.
            let bytes = b
                .get(at + 2..at + 2 + name_len)
                .ok_or_else(|| StorageError::Corrupt {
                    context: "table definition".into(),
                    detail: format!("truncated column name at {at}"),
                })?;
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| u16::from_le_bytes(*p))
                .collect();
            col.name = String::from_utf16_lossy(&units);
            at += 2 + name_len;
        }
        let _ = max_cols; // validated implicitly: num_cols <= max_cols is a real-file check
        let _ = num_var_cols; // used by the oracle comparison in step 2.4
        // Skip index blocks: real-index blocks, logical-index blocks, names.
        at += num_real_idx * 52;
        at += num_idx * 28;
        for _ in 0..num_idx {
            let name_len = le::u16(b, at, "index name_len")? as usize;
            at += 2 + name_len; // byte length
        }
        // Variable-column usage-map pointers terminate with 0xFFFF.
        if let Some(mut ptr) = b.get(at..at + 2).map(|p| u16::from_le_bytes([p[0], p[1]])) {
            // Real files sometimes end the block before the 0xFFFF
            // terminator — treat running out of bytes as the end (mdb-compat).
            while ptr != 0xFFFF && b.len() >= at + 12 {
                at += 10; // col_num u16 + two u32 usage-map pointers
                ptr = u16::from_le_bytes([b[at], b[at + 1]]);
            }
        }
        // Logical column order is the column-id (creation) order — descriptor
        // storage order can diverge from it (MSysObjects stores them
        // alphabetically), and both the null mask and the logical schema
        // follow the id space (docs/FORMAT-JET.md "Rows"). Stable sort:
        // duplicate ids keep storage order (hostile-file tolerance).
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
