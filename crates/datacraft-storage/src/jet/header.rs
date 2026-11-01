//! Jet/ACE file header decoder (step 2.2, item H5).
//!
//! Clean-room implementation from public format documentation
//! (community format reference, "first page" section):
//!
//! | Field | Offset | Size | Meaning |
//! |---|---|---|---|
//! | Magic | 0x00 | 4 | `00 01 00 00` (u32 LE = 0x100) |
//! | Product string | 0x04 | 16 | NUL-terminated `"Standard Jet DB"` (`.mdb`) / `"Standard ACE DB"` (`.accdb`) |
//! | Version | 0x14 | 4 | u32 LE: `0x00000000` Jet3, `0x00000001` Jet4, `0x00000002` ACE 2007, `0x00000103` ACE 2010 |
//!
//! The page size is **not** stored in the file; it follows from the format
//! generation: Jet 3 uses fixed 2048-byte pages, Jet 4 and ACE fixed
//! 4096-byte pages. Every deviation from the documented layout is a precise
//! [`StorageError`]. Header fields beyond 0x18 are RC4-encrypted in Jet4/ACE
//! and are not needed at this layer.

use crate::error::StorageError;
use crate::types::FormatVersion;

/// Byte signature every Jet/ACE database starts with (u32 LE = 0x100).
pub const SIGNATURE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
/// Product string field of Jet `.mdb` databases (offset 4, NUL-terminated).
pub const JET_PRODUCT: &[u8] = b"Standard Jet DB\0";
/// Product string field of ACE `.accdb` databases (offset 4, NUL-terminated).
pub const ACE_PRODUCT: &[u8] = b"Standard ACE DB\0";
/// Version dword of Jet 3 (Access 97 and earlier).
pub const JET3_VERSION: u32 = 0x0000_0000;
/// Version dword of Jet 4 (Access 2000–2003).
pub const JET4_VERSION: u32 = 0x0000_0001;
/// Version dword of ACE (Access 2007).
pub const ACE_2007_VERSION: u32 = 0x0000_0002;
/// Version dword of ACE (Access 2010).
pub const ACE_2010_VERSION: u32 = 0x0000_0103;
/// Header bytes needed by this decoder (through the version dword at 0x14..0x18).
const MIN_HEADER_LEN: usize = 0x18;

/// Decoded database header (page 0) plus derived page geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    /// Format family of the file.
    pub format: FormatVersion,
    /// Raw version dword from offset 0x14 (see the module constants).
    pub version: u32,
    /// Page size in bytes, derived from the format generation.
    pub page_size: usize,
    /// Number of pages in the file (`file_size / page_size`).
    pub page_count: usize,
}

impl FileHeader {
    /// Decodes the header from the file's first bytes plus the total file size.
    ///
    /// `first_page` needs at least `0x18` bytes (all decoded fields live
    /// there); it does not have to be a full page.
    pub fn decode(first_page: &[u8], file_size: u64) -> Result<Self, StorageError> {
        if first_page.len() < MIN_HEADER_LEN {
            return Err(StorageError::Corrupt {
                context: "file header".into(),
                detail: format!(
                    "truncated: have {} bytes, need {MIN_HEADER_LEN}",
                    first_page.len()
                ),
            });
        }
        if first_page[..4] != SIGNATURE {
            return Err(StorageError::Corrupt {
                context: "file header".into(),
                detail: format!(
                    "magic bytes mismatch at offset 0: expected {:02X?}, found {:02X?}",
                    SIGNATURE,
                    &first_page[..4]
                ),
            });
        }
        let product = &first_page[4..20];
        let is_jet = product == JET_PRODUCT;
        let is_ace = product == ACE_PRODUCT;
        if !is_jet && !is_ace {
            return Err(StorageError::Corrupt {
                context: "file header".into(),
                detail: format!(
                    "unknown product signature at offset 4: {:?}",
                    String::from_utf8_lossy(&first_page[4..19])
                ),
            });
        }
        let version = u32::from_le_bytes([
            first_page[0x14],
            first_page[0x15],
            first_page[0x16],
            first_page[0x17],
        ]);
        // Product string and format version must agree: a mixed pair is not
        // a file any real writer produces (precise Unsupported, not a guess).
        if is_ace && matches!(version, JET3_VERSION | JET4_VERSION) {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "ACE product string with Jet format version 0x{version:08x} is not a                      supported combination"
                ),
            });
        }
        if is_jet && matches!(version, ACE_2007_VERSION | ACE_2010_VERSION) {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "Jet product string with ACE format version 0x{version:08x} is not a                      supported combination"
                ),
            });
        }
        let (format, page_size) = match version {
            JET3_VERSION => (FormatVersion::Jet3, crate::jet::page::JET3_PAGE_SIZE),
            JET4_VERSION => (FormatVersion::Jet4, 4096),
            ACE_2007_VERSION | ACE_2010_VERSION => (FormatVersion::Ace, 4096),
            other => {
                return Err(StorageError::Unsupported {
                    reason: format!("format version 0x{other:08x} not supported"),
                });
            }
        };
        if file_size == 0 {
            return Err(StorageError::Corrupt {
                context: "file header".into(),
                detail: "file is empty".into(),
            });
        }
        if !file_size.is_multiple_of(page_size as u64) {
            return Err(StorageError::Corrupt {
                context: "file header".into(),
                detail: format!(
                    "file size {file_size} is not a multiple of the {page_size}-byte page size"
                ),
            });
        }
        Ok(Self {
            format,
            version,
            page_size,
            page_count: (file_size / page_size as u64) as usize,
        })
    }

    /// Byte offset of `page` in the file.
    pub fn page_offset(&self, page: u32) -> Result<u64, StorageError> {
        if page as usize >= self.page_count {
            return Err(StorageError::PageOutOfRange {
                page,
                page_count: self.page_count,
            });
        }
        Ok(page as u64 * self.page_size as u64)
    }

    /// Borrows the raw bytes of `page` from the full file buffer.
    pub fn page_slice<'a>(&self, file: &'a [u8], page: u32) -> Result<&'a [u8], StorageError> {
        let start = self.page_offset(page)? as usize;
        Ok(&file[start..start + self.page_size])
    }
}
