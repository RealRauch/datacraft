//! File-header and page-layer contract tests (step 2.2, item H5).
//!
//! The encoded facts are the publicly documented Jet/ACE file-header layout:
//! 4-byte signature `00 01 00 00`, 15-byte product string at offset 4
//! ("Standard Jet DB" / "Standard ACE DB"), page size (u16 LE) at offset 8
//! for Jet4+/ACE, generation byte at offset 0x14 (0x00 Jet3, 0x01 Jet4,
//! 0x02–0x04 ACE generations), fixed 2048-byte pages for Jet3.
//! Correctness against real files is proven by the oracle (step 2.4).
//!
//! Test-first: RED until `datacraft_storage::jet` exists.

use datacraft_storage::jet::{next_page_pointer, FileHeader};
use datacraft_storage::{FormatVersion, StorageError};

/// Builds a minimal, valid Jet/ACE header page.
fn header_page(product: &[u8], generation: u8, page_size: u16) -> Vec<u8> {
    let mut page = vec![0u8; 4096];
    page[0] = 0x00;
    page[1] = 0x01;
    page[2] = 0x00;
    page[3] = 0x00;
    let mut product_field = [0u8; 16];
    product_field[..product.len()].copy_from_slice(product);
    page[4..20].copy_from_slice(&product_field);
    page[0x08..0x0A].copy_from_slice(&page_size.to_le_bytes());
    page[0x14] = generation;
    page
}

fn jet4_header() -> Vec<u8> {
    header_page(b"Standard Jet DB\0", 0x01, 4096)
}

#[test]
fn decodes_jet4_header_with_page_count() {
    let mut file = jet4_header();
    file.extend(vec![0u8; 4096 * 2]); // two more empty pages
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid jet4 header");
    assert_eq!(header.format, FormatVersion::Jet4);
    assert_eq!(header.generation, 0x01);
    assert_eq!(header.page_size, 4096);
    assert_eq!(header.page_count, 3);
}

#[test]
fn decodes_accdb_header_with_generation() {
    let mut file = header_page(b"Standard ACE DB\0", 0x03, 4096);
    file.extend(vec![0u8; 4096]);
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid accdb header");
    assert_eq!(header.format, FormatVersion::Ace);
    assert_eq!(header.generation, 0x03);
    assert_eq!(header.page_count, 2);
}

#[test]
fn jet3_has_fixed_2048_page_size() {
    // Jet3 carries no reliable page-size field — offset 8 is ignored garbage.
    let mut page = header_page(b"Standard Jet DB\0", 0x00, 0xFFFF);
    page.extend(vec![0u8; 2048 * 2]);
    let header = FileHeader::decode(&page, page.len() as u64).expect("valid jet3 header");
    assert_eq!(header.format, FormatVersion::Jet3);
    assert_eq!(header.page_size, 2048);
    assert_eq!(header.page_count, 3);
}

#[test]
fn rejects_truncated_header() {
    let err = FileHeader::decode(&jet4_header()[..10], 10)
        .expect_err("short header must fail");
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
    let page = header_page(b"Some Other Prod", 0x01, 4096);
    let err = FileHeader::decode(&page, page.len() as u64).expect_err("unknown product must fail");
    let detail = match &err {
        StorageError::Corrupt { detail, .. } => detail.clone(),
        other => panic!("expected Corrupt, got {other:?}"),
    };
    assert!(detail.contains("Some Other Prod"), "detail: {detail}");
}

#[test]
fn rejects_unsupported_generation() {
    let page = header_page(b"Standard Jet DB\0", 0x7F, 4096);
    let err =
        FileHeader::decode(&page, page.len() as u64).expect_err("generation 0x7F must fail");
    assert!(
        matches!(err, StorageError::Unsupported { ref reason } if reason.contains("0x7f")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn rejects_implausible_page_size() {
    let page = header_page(b"Standard Jet DB\0", 0x01, 1234);
    let err = FileHeader::decode(&page, page.len() as u64).expect_err("page size 1234 must fail");
    assert!(matches!(err, StorageError::Corrupt { .. }));
}

#[test]
fn rejects_file_size_not_multiple_of_page_size() {
    let mut file = jet4_header();
    file.extend(vec![0u8; 100]);
    let err =
        FileHeader::decode(&file, file.len() as u64).expect_err("ragged file must fail");
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
    file[2 * 4096] = 0xAB; // marker on page 2
    let header = FileHeader::decode(&file, file.len() as u64).expect("valid header");
    assert_eq!(header.page_offset(2).expect("offset"), 2 * 4096);
    let page = header.page_slice(&file, 2).expect("page 2");
    assert_eq!(page[0], 0xAB);
    assert_eq!(page.len(), 4096);
    assert!(matches!(
        header.page_slice(&file, 3),
        Err(StorageError::PageOutOfRange { page: 3, page_count: 3 })
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
