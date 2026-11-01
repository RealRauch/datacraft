//! Usage-map layer (step 10.1, item H9): page allocation bookkeeping.
//!
//! Facts verified against the Access-written fixture (probe + cross-check,
//! see `docs/FORMAT-JET.md` "Usage maps (writer)"):
//!
//! - The TDEF `used_pages`/`free_pages` dwords pack `[map type in the HIGH
//!   byte][u24 LE record pointer = (page << 8) | row]`. All maps in the
//!   fixture are type `0x00` — the map is a **row in a data page** (no
//!   type-`0x05` pages exist in Access-written files).
//! - A type-0 map row is `[0x00][start_page u32][bitmap]`, LSB first, bit
//!   set = page **allocated to the table**. Rows grow with the table's
//!   window (observed: 64 and 96 bitmap bytes = 512/768-page windows,
//!   `start_page` may be non-zero, e.g. 752).
//! - The global map (page 1, row 0) has the opposite polarity: bit set =
//!   page **free / absent**; Access pre-marks headroom beyond EOF.
//!
//! Rows live in data pages owned by page `0` (usage pages; page 1 itself is
//! owned by the phony TDEF `1`) and may be rewritten in place — `(page,row)`
//! references stay valid when a row grows because rows keep their slot
//! indices (only offsets and the free-space field change).

use crate::error::StorageError;

/// A record pointer to a usage-map row: `(page << 8) | row`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapRef {
    /// Data page holding the map row.
    pub page: u32,
    /// Record slot of the row within that page.
    pub row: u8,
}

impl MapRef {
    /// Splits a TDEF `used_pages`/`free_pages` dword into its map type and
    /// row pointer. Only type `0x00` (inline row) is supported — type `0x01`
    /// indirection through `0x05` pages is not part of the writer milestone
    /// and is a precise `Unsupported`.
    pub fn split_tdef_u32(raw: u32) -> Result<(u8, Self), StorageError> {
        let kind = (raw >> 24) as u8;
        if kind != 0x00 {
            return Err(StorageError::Unsupported {
                reason: format!("usage-map type 0x{kind:02x} (indirection) is not supported"),
            });
        }
        let pointer = raw & 0x00FF_FFFF;
        Ok((
            kind,
            Self {
                page: pointer >> 8,
                row: (pointer & 0xFF) as u8,
            },
        ))
    }

    /// The u24 record pointer (`(page << 8) | row`), without the type byte.
    pub fn pack(self) -> u32 {
        (self.page << 8) | u32::from(self.row)
    }
}

/// One decoded type-0 usage map: a window of pages starting at
/// `start_page` and its allocation bitmap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageMap {
    /// First page described by the bitmap.
    pub start_page: u32,
    /// Bitmap bytes, LSB first (polarity is the caller's context: per-table
    /// maps mark allocated pages, the global map marks free pages).
    pub bitmap: Vec<u8>,
}

impl UsageMap {
    /// Decodes a type-0 map row (`[0x00][start u32][bitmap]`).
    pub fn decode_row(row: &[u8]) -> Result<Self, StorageError> {
        if row.first() != Some(&0x00) {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "usage-map row type 0x{:02x} is not supported",
                    row.first().copied().unwrap_or(0)
                ),
            });
        }
        if row.len() < 5 {
            return Err(StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!("map row of {} bytes is shorter than its header", row.len()),
            });
        }
        Ok(Self {
            start_page: u32::from_le_bytes([row[1], row[2], row[3], row[4]]),
            bitmap: row[5..].to_vec(),
        })
    }

    /// Encodes back to the row layout (byte-identical round trip).
    pub fn encode_row(&self) -> Vec<u8> {
        let mut row = Vec::with_capacity(5 + self.bitmap.len());
        row.push(0x00);
        row.extend_from_slice(&self.start_page.to_le_bytes());
        row.extend_from_slice(&self.bitmap);
        row
    }

    /// Whether the window reaches `page`.
    pub fn covers(&self, page: u32) -> bool {
        page >= self.start_page
            && usize::try_from(page - self.start_page).is_ok_and(|idx| idx < self.bitmap.len() * 8)
    }

    /// Whether the bit for `page` is set (allocated / free per polarity).
    pub fn owns(&self, page: u32) -> bool {
        self.covers(page)
            && self.bitmap[(page - self.start_page) as usize / 8] >> ((page - self.start_page) % 8)
                & 1
                == 1
    }

    /// Alias for [`UsageMap::owns`] on the global free map (bit set = page
    /// free or absent).
    pub fn owns_free(&self, page: u32) -> bool {
        self.owns(page)
    }

    /// Sets the bit for `page`, growing the window byte-granular when the
    /// page lies beyond it. Pages below `start_page` are a precise error
    /// (the writer never re-windows an existing map downwards).
    pub fn set(&mut self, page: u32) -> Result<(), StorageError> {
        self.grow_to(page)?;
        let idx = (page - self.start_page) as usize;
        self.bitmap[idx / 8] |= 1 << (idx % 8);
        Ok(())
    }

    /// Clears the bit for `page` (same window growth as [`UsageMap::set`]).
    pub fn clear(&mut self, page: u32) -> Result<(), StorageError> {
        self.grow_to(page)?;
        let idx = (page - self.start_page) as usize;
        self.bitmap[idx / 8] &= !(1 << (idx % 8));
        Ok(())
    }

    fn grow_to(&mut self, page: u32) -> Result<(), StorageError> {
        if page < self.start_page {
            return Err(StorageError::Corrupt {
                context: "usage map".into(),
                detail: format!(
                    "page {page} lies below the map window start {}",
                    self.start_page
                ),
            });
        }
        let needed = ((page - self.start_page) as usize / 8) + 1;
        if needed > self.bitmap.len() {
            self.bitmap.resize(needed, 0);
        }
        Ok(())
    }

    /// Pages with their bit set, ascending.
    pub fn marked_pages(&self) -> impl Iterator<Item = u32> + '_ {
        let start = self.start_page;
        self.bitmap.iter().enumerate().flat_map(move |(byte, &b)| {
            (0..8).filter_map(move |bit| {
                (b >> bit & 1 == 1).then_some(start + (byte * 8 + bit) as u32)
            })
        })
    }
}
