//! Jet4 index leaf pages (step 49.2, item M63).
//!
//! The engine's catalog is resolved through the `ParentIdName`/`Id` B-trees,
//! so a catalog row the writer appends is invisible until the corresponding
//! leaf carries its entry (evidence in `docs/FORMAT-JET.md`, "Fresh-create
//! template and the engine's open gates"). This module decodes, extends and
//! re-encodes single-page leaf roots byte-exactly; page splitting remains a
//! roadmap item (M15) and an overfull leaf is refused precisely.
//!
//! Layout (all facts verified against engine-written pages):
//!
//! - header: type `0x04`, `0x01`, free-space u16, parent TDEF page u32; the
//!   rest of the 0x1B-byte header is zero (including `pref_len` at 0x14).
//! - an entry-start bitmap at `0x1B..0x1E0`: bit `i` marks an entry starting
//!   at `0x1E0 + i`; the **last** set bit is the end of the last entry; the
//!   first entry at `0x1E0` is implicit (bit 0 stays clear).
//! - entries grow upward from `0x1E0`; `free = page_size - (0x1E0 + end)`.
//! - one entry is `[flag][key]…[page 3-byte BE][row 1-byte]`; the flag is
//!   `0x7F` for a sortable key and `0x00` (no key bytes) for NULL.
//! - LongInt keys are `value ^ 0x8000_0000` big-endian; text keys are the
//!   collation sort key of [`crate::jet::collation`] (self-terminated by its
//!   trailing `0x00`).
//! - entries sort by their full byte image (the NULL flag `0x00` therefore
//!   sorts before `0x7F`).

use crate::error::StorageError;
use crate::jet::collation::text_sort_key;
use crate::jet::tdef::JetTableDef;

/// Page type of an index leaf.
pub const PAGE_TYPE_LEAF: u8 = 0x04;
/// First byte of the entry area; the bitmap occupies the bytes before it.
pub const ENTRY_AREA_AT: usize = 0x1E0;
/// Bitmap start.
const BITMAP_AT: usize = 0x1B;
/// Bytes of the entry-start bitmap.
const BITMAP_LEN: usize = ENTRY_AREA_AT - BITMAP_AT;
/// Flag byte of a sortable key component.
const KEY_FLAG: u8 = 0x7F;
/// Flag byte of a NULL key component.
const NULL_FLAG: u8 = 0x00;
/// Structure tag of a physical index block (`index.rs`).
const REAL_TAG: u32 = 0x783;
/// Sentinel column id of an unused index slot.
const NO_COLUMN: u16 = 0xFFFF;
/// Byte size of one physical index block.
const REAL_INDEX_BLOCK: usize = 52;
/// Offset of `first_dp` (leaf root page) in a physical index block.
const ROOT_AT: usize = 38;
/// Offset of the 12-byte statistics records inside the TDEF block head.
const STATISTICS_AT: usize = 55;

/// `(page, row)` locator of one stored row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowPointer {
    /// Data page number.
    pub page: u32,
    /// Row slot on that page (`0..=255`).
    pub row: u8,
}

/// One decoded leaf page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafPage {
    /// Owning table definition page.
    pub parent: u32,
    /// Raw entry images in stored order (flag + keys + pointer).
    pub entries: Vec<Vec<u8>>,
}

/// One key component of a new entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPart<'a> {
    /// NULL key component (flag `0x00`, no data).
    Null,
    /// LongInteger key (`value ^ 0x8000_0000`, big-endian).
    Integer(i64),
    /// Text key (collation sort key).
    Text(&'a str),
}

/// One physical index of a table definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalIndex {
    /// Indexed column ids in key order.
    pub columns: Vec<u16>,
    /// Root page of the index's B-tree (0 = no physical index).
    pub root_page: u32,
    /// Offset of the index's row-count statistics dword, relative to the
    /// TDEF block start (page offset 8).
    pub stats_offset: usize,
}

/// Reads the physical index blocks out of a decoded TDEF.
pub fn physical_indexes(def: &JetTableDef) -> Result<Vec<PhysicalIndex>, StorageError> {
    let real = def.real_index_count as usize;
    let blocks_len = real.saturating_mul(REAL_INDEX_BLOCK);
    if def.index_section.len() < blocks_len {
        return Err(corrupt(
            "index metadata",
            "the index section is shorter than its physical block count",
        ));
    }
    let mut indexes = Vec::with_capacity(real);
    for ordinal in 0..real {
        let block =
            &def.index_section[ordinal * REAL_INDEX_BLOCK..(ordinal + 1) * REAL_INDEX_BLOCK];
        let tag = u32::from_le_bytes(block[0..4].try_into().expect("4 bytes"));
        if tag != REAL_TAG {
            return Err(corrupt(
                "index metadata",
                "unknown physical index block tag",
            ));
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
        let root_page =
            u32::from_le_bytes(block[ROOT_AT..ROOT_AT + 4].try_into().expect("4 bytes"));
        indexes.push(PhysicalIndex {
            columns,
            root_page,
            stats_offset: STATISTICS_AT + 12 * ordinal + 4,
        });
    }
    Ok(indexes)
}

/// Decodes a leaf page into its entry images.
pub fn decode_leaf(page_no: u32, page: &[u8]) -> Result<LeafPage, StorageError> {
    if page.len() < ENTRY_AREA_AT {
        return Err(corrupt(
            "index leaf",
            &format!("page {page_no} is shorter than the entry area"),
        ));
    }
    if page[0] != PAGE_TYPE_LEAF {
        return Err(corrupt(
            "index leaf",
            &format!("page {page_no} carries type 0x{:02x}, not a leaf", page[0]),
        ));
    }
    let parent = u32::from_le_bytes(page[4..8].try_into().expect("4 bytes"));
    let bitmap = &page[BITMAP_AT..ENTRY_AREA_AT];
    let mut bits = Vec::new();
    for (byte_index, byte) in bitmap.iter().enumerate() {
        for bit in 0..8usize {
            if byte >> bit & 1 != 0 {
                bits.push(byte_index * 8 + bit);
            }
        }
    }
    let area = page.len() - ENTRY_AREA_AT;
    if let Some(last) = bits.last() {
        if *last > area {
            return Err(corrupt(
                "index leaf",
                &format!("page {page_no} marks an entry end at {last} beyond the entry area"),
            ));
        }
    }
    let declared_free = u16::from_le_bytes([page[2], page[3]]) as usize;
    let end = bits.last().copied().unwrap_or(0);
    let computed_free = page.len() - (ENTRY_AREA_AT + end);
    if declared_free != computed_free {
        return Err(corrupt(
            "index leaf",
            &format!(
                "page {page_no} declares {declared_free} free bytes but the entry area ends at {end}"
            ),
        ));
    }
    let mut starts = Vec::with_capacity(bits.len());
    if !bits.is_empty() {
        starts.push(0);
        starts.extend_from_slice(&bits[..bits.len() - 1]);
    }
    let mut entries = Vec::with_capacity(starts.len());
    for (position, start) in starts.iter().enumerate() {
        let stop = starts.get(position + 1).copied().unwrap_or(end);
        if stop <= *start || *start >= area {
            return Err(corrupt(
                "index leaf",
                &format!("page {page_no} carries an empty or out-of-range entry at {start}"),
            ));
        }
        let Some(bytes) = page.get(ENTRY_AREA_AT + start..ENTRY_AREA_AT + stop) else {
            return Err(corrupt(
                "index leaf",
                &format!("page {page_no} carries a truncated entry at {start}"),
            ));
        };
        if bytes.len() < 5 {
            return Err(corrupt(
                "index leaf",
                &format!(
                    "page {page_no} carries a {} byte entry (shorter than flag + pointer)",
                    bytes.len()
                ),
            ));
        }
        entries.push(bytes.to_vec());
    }
    Ok(LeafPage { parent, entries })
}

/// Encodes a leaf page. An entry area that does not fit answers
/// `Unsupported` — multi-page indexes are a roadmap item, never a silent
/// truncation.
pub fn encode_leaf(leaf: &LeafPage, page_size: usize) -> Result<Vec<u8>, StorageError> {
    if page_size < ENTRY_AREA_AT {
        return Err(StorageError::Unsupported {
            reason: format!("a page of {page_size} bytes cannot hold an index leaf"),
        });
    }
    let mut page = vec![0u8; page_size];
    page[0] = PAGE_TYPE_LEAF;
    page[1] = 0x01;
    page[4..8].copy_from_slice(&leaf.parent.to_le_bytes());
    let mut bits: Vec<usize> = Vec::with_capacity(leaf.entries.len());
    let mut at = ENTRY_AREA_AT;
    for (position, entry) in leaf.entries.iter().enumerate() {
        if entry.len() < 5 {
            return Err(corrupt(
                "index leaf",
                &format!(
                    "entry {position} is {} bytes (shorter than flag + pointer)",
                    entry.len()
                ),
            ));
        }
        if position > 0 {
            bits.push(at - ENTRY_AREA_AT);
        }
        let Some(slot) = page.get_mut(at..at + entry.len()) else {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "the index leaf needs {} bytes but its entry area holds {} — \
                     multi-page index splitting is not implemented yet (roadmap M15)",
                    at + entry.len() - ENTRY_AREA_AT,
                    page_size - ENTRY_AREA_AT
                ),
            });
        };
        slot.copy_from_slice(entry);
        at += entry.len();
    }
    if !leaf.entries.is_empty() {
        bits.push(at - ENTRY_AREA_AT);
    }
    for bit in bits {
        if bit / 8 >= BITMAP_LEN {
            return Err(StorageError::Unsupported {
                reason: format!("index leaf entry offset {bit} exceeds the bitmap range"),
            });
        }
        page[BITMAP_AT + bit / 8] |= 1 << (bit % 8);
    }
    let free = (page_size - at) as u16;
    page[2..4].copy_from_slice(&free.to_le_bytes());
    Ok(page)
}

/// Builds one leaf entry image from its key components and the row pointer.
pub fn leaf_entry(keys: &[KeyPart<'_>], pointer: RowPointer) -> Result<Vec<u8>, StorageError> {
    if keys.is_empty() {
        return Err(StorageError::Unsupported {
            reason: "an index entry needs at least one key column".to_owned(),
        });
    }
    let mut entry = Vec::with_capacity(keys.len() * 6 + 4);
    for key in keys {
        match key {
            KeyPart::Null => entry.push(NULL_FLAG),
            KeyPart::Integer(value) => {
                entry.push(KEY_FLAG);
                let ordered = (*value as i32 as u32) ^ 0x8000_0000;
                entry.extend_from_slice(&ordered.to_be_bytes());
            }
            KeyPart::Text(text) => {
                entry.push(KEY_FLAG);
                entry.extend_from_slice(&text_sort_key(text)?);
            }
        }
    }
    entry.extend_from_slice(&pointer_bytes(pointer)?);
    Ok(entry)
}

/// Reads the trailing row pointer of an entry image.
pub fn entry_pointer(entry: &[u8]) -> Result<RowPointer, StorageError> {
    if entry.len() < 5 {
        return Err(corrupt(
            "index leaf",
            "an entry shorter than its flag and row pointer",
        ));
    }
    let at = entry.len() - 4;
    Ok(RowPointer {
        page: u32::from_be_bytes([0, entry[at], entry[at + 1], entry[at + 2]]),
        row: entry[at + 3],
    })
}

/// The four stored pointer bytes: page big-endian in three bytes, then row.
fn pointer_bytes(pointer: RowPointer) -> Result<[u8; 4], StorageError> {
    if pointer.page > 0x00FF_FFFF {
        return Err(StorageError::Unsupported {
            reason: format!(
                "page {} does not fit the three-byte index pointer",
                pointer.page
            ),
        });
    }
    let page = pointer.page.to_be_bytes();
    Ok([page[1], page[2], page[3], pointer.row])
}

fn corrupt(context: &str, detail: &str) -> StorageError {
    StorageError::Corrupt {
        context: context.to_owned(),
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests;
