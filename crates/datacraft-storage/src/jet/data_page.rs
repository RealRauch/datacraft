//! Data page decoder (step 2.3, item H6).
//!
//! Layout reference: `docs/FORMAT-JET.md` ("Data pages"). Records are stored
//! back-to-front; the offset list carries deletion/overflow flags in its high
//! bits.

use crate::error::StorageError;
use crate::jet::page::PAGE_TYPE_DATA;

/// A decoded data page: owning TDEF page and the record byte ranges.
#[derive(Debug)]
pub struct DataPage {
    /// TDEF page of the owning table (or raw `"LVAL"` bytes for long-value pages).
    pub owner: u32,
    /// Slots in raw order as `(start, end, raw_offset)` — raw indices stay
    /// stable because overflow pointers name them. Slots flagged `0x8000`
    /// (deleted rows and overflow targets) are skipped during iteration but
    /// keep valid coordinates; `0x4000` marks overflow slots holding a
    /// 4-byte row pointer instead of data.
    pub records: Vec<(usize, usize, u16)>,
}

impl DataPage {
    /// Decodes the page header and record index from a full page slice.
    pub fn decode(page: &[u8]) -> Result<Self, StorageError> {
        if page.len() < 14 {
            return Err(StorageError::Corrupt {
                context: "data page".into(),
                detail: format!("truncated: have {} bytes, need 14", page.len()),
            });
        }
        if page[0] != PAGE_TYPE_DATA {
            return Err(StorageError::Corrupt {
                context: "data page".into(),
                detail: format!("page type 0x{:02x} is not a data page", page[0]),
            });
        }
        let owner = u32::from_le_bytes([page[4], page[5], page[6], page[7]]);
        let num_rows = u16::from_le_bytes([page[12], page[13]]) as usize;
        if 14 + num_rows * 2 > page.len() {
            return Err(StorageError::Corrupt {
                context: "data page".into(),
                detail: format!("record count {num_rows} exceeds the page's offset table capacity"),
            });
        }
        let mut records = Vec::with_capacity(num_rows);
        let mut end = page.len();
        for i in 0..num_rows {
            let at = 14 + i * 2;
            let raw = u16::from_le_bytes([page[at], page[at + 1]]);
            let start = (raw & 0x1FFF) as usize;
            if raw & 0x4000 != 0 {
                // Overflow slot: holds a 4-byte row pointer instead of data.
                records.push((start, start + 4, raw));
                end = start;
                continue;
            }
            if start >= end {
                return Err(StorageError::Corrupt {
                    context: "data page".into(),
                    detail: format!(
                        "record {i} offset {start} is not below the previous record end {end}"
                    ),
                });
            }
            records.push((start, end, raw));
            end = start;
        }
        Ok(Self { owner, records })
    }

    /// Borrows the raw byte range of slot `i`.
    pub fn record(&self, i: usize) -> (usize, usize) {
        let (start, end, _) = self.records[i];
        (start, end)
    }

    /// Whether slot `i` is an overflow slot (its 4 bytes are a row pointer).
    pub fn is_overflow(&self, i: usize) -> bool {
        self.records[i].2 & 0x4000 != 0
    }
}
