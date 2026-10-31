//! Page-level primitives (steps 2.2–2.3, item H5/H6).
//!
//! Pages are 0-indexed regions of `page_size` bytes; page N begins at byte
//! offset `N * page_size`. With the exception of the header page (page 0),
//! the **first byte of every page is the page type** (see
//! `docs/FORMAT-JET.md`).

/// Page size of Jet 3 databases (fixed by the format, no header field).
pub const JET3_PAGE_SIZE: usize = 2048;
/// Data page (table rows and LVAL long-value records).
pub const PAGE_TYPE_DATA: u8 = 0x01;
/// Table definition page.
pub const PAGE_TYPE_TDEF: u8 = 0x02;

/// Reads the page-type byte from the start of a page.
///
/// # Panics
/// Panics if `page` is empty — the page layer always hands out full-size
/// page slices, so this is a programming error.
pub fn page_type(page: &[u8]) -> u8 {
    page[0]
}
