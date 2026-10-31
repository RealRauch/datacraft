//! File-header and page-layer contract tests (step 2.2, item H5).
//!
//! The encoded facts come from the public format documentation ("first
//! page"): magic `00 01 00 00`, 16-byte NUL-terminated product string at
//! offset 4, u32-LE version dword at 0x14 (0x00 Jet3, 0x01 Jet4, 0x02 ACE
//! 2007, 0x103 ACE 2010), page size derived from the generation (Jet3 2048,
//! Jet4/ACE 4096). Correctness against real files is proven by the oracle
//! (step 2.4).
//!
//! Test-first: RED until `datacraft_storage::jet` exists.

use datacraft_storage::jet::{
    ACE_2007_VERSION, ACE_2010_VERSION, FileHeader, JET4_VERSION, next_page_pointer,
};
use datacraft_storage::{FormatVersion, StorageError};

/// Builds a minimal, valid Jet/ACE header page.
fn header_page(product: &[u8], version: u32) -> Vec<u8> {
    let mut page = vec![0u8; 4096];
    page[0] = 0x00;
    page[1] = 0x01;
    page[2] = 0x00;
    page[3] = 0x00;
    let mut product_field = [0u8; 16];
    product_field[..product.len()].copy_from_slice(product);
    page[4..20].copy_from_slice(&product_field);
    page[0x14..0x18].copy_from_slice(&version.to_le_bytes());
    page
}

fn jet4_header() -> Vec<u8> {
    header_page(b"Standard Jet DB\0", JET4_VERSION)
}

#[test]
fn decodes_jet4_header_with_page_count() {
    let mut file = jet4_header();
    file.extend(vec![0u8; 4096 * 2]); // two more empty pages
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid jet4 header");
    assert_eq!(header.format, FormatVersion::Jet4);
    assert_eq!(header.version, JET4_VERSION);
    assert_eq!(header.page_size, 4096);
    assert_eq!(header.page_count, 3);
}

#[test]
fn decodes_accdb_2007_header() {
    let mut file = header_page(b"Standard ACE DB\0", ACE_2007_VERSION);
    file.extend(vec![0u8; 4096]);
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid accdb header");
    assert_eq!(header.format, FormatVersion::Ace);
    assert_eq!(header.version, ACE_2007_VERSION);
    assert_eq!(header.page_size, 4096);
}

#[test]
fn decodes_accdb_2010_header() {
    let file = header_page(b"Standard ACE DB\0", ACE_2010_VERSION);
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid accdb 2010 header");
    assert_eq!(header.format, FormatVersion::Ace);
    assert_eq!(header.version, ACE_2010_VERSION);
    assert_eq!(header.page_size, 4096);
}

#[test]
fn jet3_has_fixed_2048_page_size() {
    let mut page = header_page(b"Standard Jet DB\0", 0x0000_0000);
    page.truncate(2048); // Jet3 pages are 2048 bytes
    page.extend(vec![0u8; 2048 * 2]);
    let header = FileHeader::decode(&page, page.len() as u64).expect("valid jet3 header");
    assert_eq!(header.format, FormatVersion::Jet3);
    assert_eq!(header.page_size, 2048);
    assert_eq!(header.page_count, 3);
}

#[test]
fn rejects_truncated_header() {
    let err = FileHeader::decode(&jet4_header()[..10], 10).expect_err("short header must fail");
    assert!(matches!(err, StorageError::Corrupt { ref context, .. } if context.contains("header")));
}

#[test]
fn rejects_bad_magic_bytes() {
    let mut page = jet4_header();
    page[1] = 0x02;
    let err = FileHeader::decode(&page, page.len() as u64).expect_err("bad magic must fail");
    let detail = match &err {
        StorageError::Corrupt { detail, .. } => detail.clone(),
        other => panic!("expected Corrupt, got {other:?}"),
    };
    assert!(detail.contains("magic"), "detail: {detail}");
}

#[test]
fn rejects_unknown_product_string() {
    let page = header_page(b"Some Other Prod", JET4_VERSION);
    let err = FileHeader::decode(&page, page.len() as u64).expect_err("unknown product must fail");
    let detail = match &err {
        StorageError::Corrupt { detail, .. } => detail.clone(),
        other => panic!("expected Corrupt, got {other:?}"),
    };
    assert!(detail.contains("Some Other Prod"), "detail: {detail}");
}

#[test]
fn rejects_unsupported_version() {
    let page = header_page(b"Standard Jet DB\0", 0x0000_007F);
    let err = FileHeader::decode(&page, page.len() as u64).expect_err("version 0x7f must fail");
    assert!(
        matches!(err, StorageError::Unsupported { ref reason } if reason.contains("0x0000007f")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn rejects_file_size_not_multiple_of_page_size() {
    let mut file = jet4_header();
    file.extend(vec![0u8; 100]);
    let err = FileHeader::decode(&file, file.len() as u64).expect_err("ragged file must fail");
    let detail = match &err {
        StorageError::Corrupt { detail, .. } => detail.clone(),
        other => panic!("expected Corrupt, got {other:?}"),
    };
    assert!(detail.contains("multiple"), "detail: {detail}");
}

#[test]
fn rejects_empty_file() {
    let err = FileHeader::decode(&[], 0).expect_err("empty file must fail");
    assert!(matches!(err, StorageError::Corrupt { .. }));
}

#[test]
fn page_addressing_roundtrip_and_bounds() {
    let mut file = jet4_header();
    file.extend(vec![0u8; 4096]);
    file.extend(vec![0u8; 4096]);
    file[2 * 4096] = 0xAB; // marker on page 2
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid header");
    assert_eq!(header.page_offset(2).expect("offset"), 2 * 4096);
    let page = header.page_slice(&file, 2).expect("page 2");
    assert_eq!(page[0], 0xAB);
    assert_eq!(page.len(), 4096);
    assert!(matches!(
        header.page_slice(&file, 3),
        Err(StorageError::PageOutOfRange {
            page: 3,
            page_count: 3
        })
    ));
}

#[test]
fn next_page_pointer_is_little_endian_u32() {
    let mut page = vec![0u8; 4096];
    page[0] = 0x11;
    page[1] = 0x22;
    page[2] = 0x33;
    page[3] = 0x44;
    assert_eq!(next_page_pointer(&page), 0x44332211);
}
