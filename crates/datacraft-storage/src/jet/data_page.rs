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
    /// Records in page order as `(start, end)` byte ranges into the page.
    /// Deleted rows are **excluded**; overflow rows produce an error in
    /// [`DataPage::decode`].
    pub records: Vec<(usize, usize)>,
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
            if raw & 0x4000 != 0 {
                return Err(StorageError::Unsupported {
                    reason: format!(
                        "overflow row pointer on record {i} — overflow pages are not read yet"
                    ),
                });
            }
            if raw & 0x8000 != 0 {
                continue; // deleted row
            }
            let start = (raw & 0x1FFF) as usize;
            if start >= end {
                return Err(StorageError::Corrupt {
                    context: "data page".into(),
                    detail: format!(
                        "record {i} offset {start} is not below the previous record end {end}"
                    ),
                });
            }
            records.push((start, end));
            end = start;
        }
        Ok(Self { owner, records })
    }

    /// Borrows the raw bytes of record `i`.
    pub fn record<'a>(&self, page: &'a [u8], i: usize) -> &'a [u8] {
        let (start, end) = self.records[i];
        &page[start..end]
    }
}
