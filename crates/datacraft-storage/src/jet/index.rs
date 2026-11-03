//! Jet TDEF index metadata (step 32.2, item M15).
//!
//! Facts per `docs/FORMAT-JET.md` and the public MDB Tools `HACKING`
//! documentation, verified against the pinned fixtures: after the column
//! names the TDEF carries `num_real_idx` 52-byte physical index blocks,
//! then `num_idx` 28-byte logical index blocks, then one u16-length +
//! UCS-2 name per logical index. Physical index B-tree pages are not
//! written yet (roadmap); the metadata is enough for engines to see the
//! index definition and for DataCraft to enforce uniqueness.

use datacraft_core::schema::IndexMode;

use crate::error::StorageError;

use super::tdef::JetColumn;

/// Byte size of one physical index block.
pub const REAL_INDEX_BLOCK: usize = 52;
/// Byte size of one logical index block.
pub const LOGICAL_INDEX_BLOCK: usize = 28;
/// Structure tag of a physical index block (1923).
const REAL_TAG: u32 = 0x783;
/// Structure tag of a logical index block (1625).
const LOGICAL_TAG: u32 = 0x659;
/// Sentinel column id of an unused index slot.
const NO_COLUMN: u16 = 0xFFFF;

/// Physical-index flags (offset 46).
pub mod flags {
    /// Unique index.
    pub const UNIQUE: u8 = 0x01;
    /// Ignore NULLs for uniqueness (multiple NULLs are allowed).
    pub const IGNORE_NULLS: u8 = 0x02;
    /// Required index.
    pub const REQUIRED: u8 = 0x08;
    /// Marker set by Access 2000+ on every index it writes.
    pub const ACCESS: u8 = 0x80;
}

/// One decoded logical index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetIndex {
    /// Index name.
    pub name: String,
    /// Indexed column ids in order (empty for a malformed index).
    pub columns: Vec<u16>,
    /// Unique index (`NoDuplicates`).
    pub unique: bool,
    /// Required index flag.
    pub required: bool,
    /// Primary-key logical index.
    pub primary: bool,
    /// Foreign-key logical index.
    pub foreign: bool,
}

impl JetIndex {
    /// The single indexed column, when the index is single-column.
    pub fn single_column(&self) -> Option<u16> {
        match self.columns.as_slice() {
            [column] => Some(*column),
            _ => None,
        }
    }

    /// Maps the index onto the domain field property of its single column.
    pub fn index_mode(&self) -> Option<IndexMode> {
        self.single_column().map(|_| {
            if self.unique {
                IndexMode::NoDuplicates
            } else {
                IndexMode::DuplicatesOk
            }
        })
    }
}

/// Builds the TDEF index section from the columns' domain `IndexMode`.
/// The caller has validated indexability (`FieldProperties::validate_for`).
pub fn build_section(columns: &[JetColumn]) -> (Vec<u8>, u32, u32) {
    let indexes: Vec<JetIndex> = columns
        .iter()
        .filter(|column| column.indexed != IndexMode::None)
        .map(|column| JetIndex {
            name: column.name.clone(),
            columns: vec![column.column_id],
            unique: column.indexed == IndexMode::NoDuplicates,
            required: false,
            primary: false,
            foreign: false,
        })
        .collect();
    encode_section(&indexes)
}

/// Encodes indexes into the TDEF section; returns `(bytes, num_idx,
/// num_real_idx)`.
pub fn encode_section(indexes: &[JetIndex]) -> (Vec<u8>, u32, u32) {
    let mut out = Vec::with_capacity(indexes.len() * (REAL_INDEX_BLOCK + LOGICAL_INDEX_BLOCK + 64));
    for index in indexes {
        let mut block = vec![0u8; REAL_INDEX_BLOCK];
        block[0..4].copy_from_slice(&REAL_TAG.to_le_bytes());
        for (slot, column) in index.columns.iter().take(10).enumerate() {
            let at = 4 + slot * 3;
            block[at..at + 2].copy_from_slice(&column.to_le_bytes());
            block[at + 2] = 1; // ascending
        }
        for slot in index.columns.len().min(10)..10 {
            let at = 4 + slot * 3;
            block[at..at + 2].copy_from_slice(&NO_COLUMN.to_le_bytes());
        }
        let mut index_flags = flags::ACCESS | flags::IGNORE_NULLS;
        if index.unique {
            index_flags |= flags::UNIQUE;
        }
        if index.required {
            index_flags |= flags::REQUIRED;
        }
        block[46] = index_flags;
        out.extend_from_slice(&block);
    }
    for (number, index) in indexes.iter().enumerate() {
        let mut block = vec![0u8; LOGICAL_INDEX_BLOCK];
        block[0..4].copy_from_slice(&LOGICAL_TAG.to_le_bytes());
        block[4..8].copy_from_slice(&(number as u32).to_le_bytes());
        block[8..12].copy_from_slice(&(number as u32).to_le_bytes());
        block[13..17].copy_from_slice(&u32::MAX.to_le_bytes());
        block[23] = u8::from(index.primary) | (u8::from(index.foreign) << 1);
        out.extend_from_slice(&block);
    }
    for index in indexes {
        let units: Vec<u16> = index.name.encode_utf16().collect();
        out.extend_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        for unit in units {
            out.extend_from_slice(&unit.to_le_bytes());
        }
    }
    (out, indexes.len() as u32, indexes.len() as u32)
}

/// Decodes the index section and returns the indexes plus the consumed
/// byte length (the raw prefix is preserved for unrelated rewrites).
pub fn decode_section(
    section: &[u8],
    index_count: usize,
    real_count: usize,
) -> Result<(Vec<JetIndex>, usize), StorageError> {
    let blocks_len = real_count * REAL_INDEX_BLOCK + index_count * LOGICAL_INDEX_BLOCK;
    if section.len() < blocks_len {
        return Err(corrupt("index section is shorter than its block counts"));
    }
    let mut names_at = blocks_len;
    let mut names = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        let len = le_u16(section, names_at, "index name length")? as usize;
        let bytes = section
            .get(names_at + 2..names_at + 2 + len)
            .ok_or_else(|| corrupt("truncated index name"))?;
        names.push(utf16(bytes));
        names_at += 2 + len;
    }
    let mut indexes = Vec::with_capacity(index_count);
    for (number, name) in names.into_iter().enumerate() {
        let logical_at = real_count * REAL_INDEX_BLOCK + number * LOGICAL_INDEX_BLOCK;
        let logical = &section[logical_at..logical_at + LOGICAL_INDEX_BLOCK];
        let tag = u32::from_le_bytes(logical[0..4].try_into().expect("4 bytes"));
        if tag != LOGICAL_TAG {
            return Err(corrupt("unknown logical index block tag"));
        }
        let real = u32::from_le_bytes(logical[8..12].try_into().expect("4 bytes")) as usize;
        let flags = logical[23];
        let Some(block) = section.get(real * REAL_INDEX_BLOCK..(real + 1) * REAL_INDEX_BLOCK)
        else {
            return Err(corrupt("logical index references a missing real block"));
        };
        let real_tag = u32::from_le_bytes(block[0..4].try_into().expect("4 bytes"));
        if real_tag != REAL_TAG {
            return Err(corrupt("unknown real index block tag"));
        }
        let mut columns = Vec::new();
        for slot in 0..10 {
            let at = 4 + slot * 3;
            let column = u16::from_le_bytes(block[at..at + 2].try_into().expect("2 bytes"));
            if column == NO_COLUMN {
                break;
            }
            columns.push(column);
        }
        let index_flags = block[46];
        indexes.push(JetIndex {
            name,
            columns,
            unique: index_flags & flags::UNIQUE != 0,
            required: index_flags & flags::REQUIRED != 0,
            primary: flags & 0x01 != 0,
            foreign: flags & 0x02 != 0,
        });
    }
    Ok((indexes, names_at))
}

fn le_u16(bytes: &[u8], at: usize, what: &str) -> Result<u16, StorageError> {
    bytes
        .get(at..at + 2)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
        .ok_or_else(|| corrupt(&format!("truncated {what}")))
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

fn corrupt(detail: &str) -> StorageError {
    StorageError::Corrupt {
        context: "index metadata".into(),
        detail: detail.to_owned(),
    }
}
