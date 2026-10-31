//! Page-level primitives (step 2.2, item H5).
//!
//! Pages are 0-indexed regions of `page_size` bytes; page N begins at byte
//! offset `N * page_size`. Jet4+/ACE pages (except the header page) begin
//! with a 4-byte little-endian "next page" pointer (0 = none).

/// Page size of Jet 3 databases (fixed by the format, no header field).
pub const JET3_PAGE_SIZE: usize = 2048;

/// Reads the 4-byte little-endian next-page pointer from a page's start.
///
/// # Panics
/// Panics if `page` is shorter than 4 bytes — the page layer always hands
/// out full-size page slices, so this is a programming error.
pub fn next_page_pointer(page: &[u8]) -> u32 {
    u32::from_le_bytes([page[0], page[1], page[2], page[3]])
}
