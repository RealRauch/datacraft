//! Leaf-page contracts (step 49.2, item M63).
//!
//! Reference bytes are engine output: `fresh_template.mdb` pages 7 and 8 are
//! the committed minimal engine-written catalog (provenance
//! `tools/dao-oracle/oracle.ps1 new-db`), and `ENG_TABLE_P7` is the 192-byte
//! entry region observed after a DAO `CREATE TABLE Foo` grew the same leaf
//! (`eng-table.mdb`, recorded 2026-11-07). The round-trip test proves
//! decode/encode are exact inverses on the real page.

use crate::error::StorageError;
use crate::jet::index_pages::{
    KeyPart, LeafPage, PhysicalIndex, RowPointer, decode_leaf, encode_leaf, entry_pointer,
    leaf_entry, physical_indexes,
};
use crate::jet::tdef::JetTableDef;

/// The embedded engine-written template (`src/jet/fresh_template.mdb`).
const FRESH_TEMPLATE: &[u8] = include_bytes!("../fresh_template.mdb");
const PAGE_SIZE: usize = 4096;
const CATALOG_TDEF_PAGE: u32 = 2;

/// Entry area of `eng-table.mdb` page 7 after `CREATE TABLE Foo` — observed
/// engine output (192 bytes, free field 3424).
const ENG_TABLE_P7: [u8; 192] = [
    0x7f, 0x8f, 0x00, 0x00, 0x00, 0x7f, 0x4f, 0x4a, 0x6d, 0x4a, 0x4c, 0x4a, 0x6b, 0x51, 0x6b, 0x01,
    0x00, 0x00, 0x00, 0x11, 0x01, 0x7f, 0x8f, 0x00, 0x00, 0x00, 0x7f, 0x69, 0x51, 0x5e, 0x4a, 0x6d,
    0x59, 0x64, 0x62, 0x6b, 0x57, 0x59, 0x66, 0x6b, 0x01, 0x00, 0x00, 0x00, 0x11, 0x02, 0x7f, 0x8f,
    0x00, 0x00, 0x00, 0x7f, 0x6d, 0x4a, 0x4c, 0x5e, 0x51, 0x6b, 0x01, 0x00, 0x00, 0x00, 0x11, 0x00,
    0x7f, 0x8f, 0x00, 0x00, 0x01, 0x7f, 0x53, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x11, 0x08, 0x7f,
    0x8f, 0x00, 0x00, 0x01, 0x7f, 0x60, 0x6b, 0x76, 0x6b, 0x4a, 0x4d, 0x51, 0x6b, 0x01, 0x00, 0x00,
    0x00, 0x11, 0x05, 0x7f, 0x8f, 0x00, 0x00, 0x01, 0x7f, 0x60, 0x6b, 0x76, 0x6b, 0x64, 0x4c, 0x5b,
    0x51, 0x4d, 0x6d, 0x6b, 0x01, 0x00, 0x00, 0x00, 0x11, 0x04, 0x7f, 0x8f, 0x00, 0x00, 0x01, 0x7f,
    0x60, 0x6b, 0x76, 0x6b, 0x68, 0x6f, 0x51, 0x69, 0x59, 0x51, 0x6b, 0x01, 0x00, 0x00, 0x00, 0x11,
    0x06, 0x7f, 0x8f, 0x00, 0x00, 0x01, 0x7f, 0x60, 0x6b, 0x76, 0x6b, 0x69, 0x51, 0x5e, 0x4a, 0x6d,
    0x59, 0x64, 0x62, 0x6b, 0x57, 0x59, 0x66, 0x6b, 0x01, 0x00, 0x00, 0x00, 0x11, 0x07, 0x7f, 0x8f,
    0x00, 0x00, 0x02, 0x7f, 0x60, 0x6b, 0x76, 0x6b, 0x4f, 0x4c, 0x01, 0x00, 0x00, 0x00, 0x11, 0x03,
];

fn page(number: u32) -> &'static [u8] {
    let at = number as usize * PAGE_SIZE;
    &FRESH_TEMPLATE[at..at + PAGE_SIZE]
}

fn catalog() -> JetTableDef {
    JetTableDef::decode(FRESH_TEMPLATE, CATALOG_TDEF_PAGE).expect("template catalog decodes")
}

#[test]
fn template_leaves_decode_to_the_engine_entries() {
    let parent_id_name = decode_leaf(7, page(7)).expect("page 7 decodes");
    assert_eq!(parent_id_name.parent, CATALOG_TDEF_PAGE);
    assert_eq!(parent_id_name.entries.len(), 8, "one entry per catalog row");
    assert_eq!(
        parent_id_name.entries[0],
        [
            0x7F, 0x8F, 0x00, 0x00, 0x00, 0x7F, 0x4F, 0x4A, 0x6D, 0x4A, 0x4C, 0x4A, 0x6B, 0x51,
            0x6B, 0x01, 0x00, 0x00, 0x00, 0x11, 0x01
        ],
        "Databases entry is byte-exact"
    );
    assert_eq!(
        entry_pointer(&parent_id_name.entries[0]).expect("pointer"),
        RowPointer { page: 17, row: 1 }
    );

    let id = decode_leaf(8, page(8)).expect("page 8 decodes");
    assert_eq!(id.entries.len(), 8);
    assert_eq!(
        id.entries[7],
        [0x7F, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x03],
        "the MSysDb row's Id 0x10000000 is the last Id entry"
    );
    assert_eq!(
        entry_pointer(&id.entries[0]).expect("pointer"),
        RowPointer { page: 17, row: 4 }
    );
}

#[test]
fn decode_and_encode_are_exact_inverses_on_engine_pages() {
    for number in [7, 8, 12] {
        let source = page(number);
        let decoded = decode_leaf(number, source).expect("leaf decodes");
        let encoded = encode_leaf(&decoded, PAGE_SIZE).expect("leaf encodes");
        assert_eq!(encoded, source, "page {number} round-trips byte-exact");
    }
}

#[test]
fn appending_the_engine_added_table_reproduces_its_grown_leaf() {
    let mut leaf = decode_leaf(7, page(7)).expect("page 7 decodes");
    let foo = leaf_entry(
        &[KeyPart::Integer(0x0F00_0001), KeyPart::Text("Foo")],
        RowPointer { page: 17, row: 8 },
    )
    .expect("Foo entry");
    leaf.entries.push(foo);
    leaf.entries.sort();
    let encoded = encode_leaf(&leaf, PAGE_SIZE).expect("grown leaf encodes");
    assert_eq!(
        &encoded[0x1E0..0x1E0 + ENG_TABLE_P7.len()],
        &ENG_TABLE_P7[..],
        "the entry region matches the engine's grown leaf"
    );
    assert_eq!(
        u16::from_le_bytes([encoded[2], encoded[3]]),
        3424,
        "the free-space field matches the engine's value"
    );
}

#[test]
fn entry_parts_encode_as_observed() {
    let pointer = RowPointer { page: 17, row: 8 };
    assert_eq!(
        leaf_entry(&[KeyPart::Null], pointer).expect("null"),
        [0x00, 0x00, 0x00, 0x11, 0x08]
    );
    assert_eq!(
        leaf_entry(&[KeyPart::Integer(2)], pointer).expect("int"),
        [0x7F, 0x80, 0x00, 0x00, 0x02, 0x00, 0x00, 0x11, 0x08]
    );
    assert_eq!(
        leaf_entry(&[KeyPart::Integer(-1)], pointer).expect("negative int"),
        [0x7F, 0x7F, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x11, 0x08]
    );
    assert_eq!(
        leaf_entry(&[KeyPart::Text("Foo")], pointer).expect("text"),
        [0x7F, 0x53, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x11, 0x08]
    );
    assert_eq!(
        leaf_entry(
            &[KeyPart::Integer(0x0F00_0001), KeyPart::Text("Foo")],
            pointer
        )
        .expect("composite"),
        [
            0x7F, 0x8F, 0x00, 0x00, 0x01, 0x7F, 0x53, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x11,
            0x08
        ]
    );
}

/// Scalar keys are sort-ordered copies of the stored value (DAO probe batch
/// 2026-11-07: one table per type, one index, observed leaf keys).
#[test]
fn scalar_key_parts_encode_as_observed() {
    let pointer = RowPointer { page: 22, row: 1 };
    let check = |part: KeyPart<'_>, expected: &[u8]| {
        assert_eq!(
            leaf_entry(&[part], pointer).expect("entry"),
            [expected, &[0x00, 0x00, 0x16, 0x01]].concat()
        );
    };
    check(KeyPart::Byte(0xC8), &[0x7F, 0xC8]);
    check(KeyPart::Boolean(true), &[0x7F, 0xFF]);
    check(KeyPart::Boolean(false), &[0x7F, 0x00]);
    check(KeyPart::SmallInteger(-1), &[0x7F, 0x7F, 0xFF]);
    check(KeyPart::SmallInteger(1), &[0x7F, 0x80, 0x01]);
    check(KeyPart::Integer(-1), &[0x7F, 0x7F, 0xFF, 0xFF, 0xFF]);
    check(
        KeyPart::Currency(15_000),
        &[0x7F, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3A, 0x98],
    );
    check(
        KeyPart::Currency(-15_000),
        &[0x7F, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xC5, 0x68],
    );
    check(
        KeyPart::Single(1.5),
        &[0x7F, 0xBF, 0xC0, 0x00, 0x00],
    );
    check(
        KeyPart::Double(1.5),
        &[0x7F, 0xBF, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );
    // 2020-01-01 as an OLE automation date (43831.0).
    check(
        KeyPart::DateTime(43_831.0),
        &[0x7F, 0xC0, 0xE5, 0x66, 0xE0, 0x00, 0x00, 0x00, 0x00],
    );
    check(
        KeyPart::Guid([
            0xFF, 0x19, 0x96, 0x6F, 0x86, 0x8B, 0x11, 0xD0, 0xB4, 0x2D, 0x00, 0xC0, 0x4F, 0xC9,
            0x64, 0xFF,
        ]),
        &[
            0x7F, 0x6F, 0x96, 0x19, 0xFF, 0x8B, 0x86, 0xD0, 0x11, 0x09, 0xB4, 0x2D, 0x00, 0xC0,
            0x4F, 0xC9, 0x64, 0xFF, 0x08,
        ],
    );
}

#[test]
fn physical_indexes_carry_columns_roots_and_statistics_slots() {
    let indexes = physical_indexes(&catalog()).expect("catalog indexes");
    assert_eq!(indexes.len(), 2);
    let parent_id_name = indexes
        .iter()
        .find(|index| index.columns == [1, 2])
        .expect("ParentIdName");
    assert_eq!(parent_id_name.root_page, 7);
    let id = indexes
        .iter()
        .find(|index| index.columns == [0])
        .expect("Id");
    assert_eq!(id.root_page, 8);
    // The row-count dword of each index's 12-byte statistics record sits at
    // block offset 55 + 12*i + 4 (observed: 8 for the template's 8 rows).
    assert_eq!(id.stats_offset, 55 + 12 + 4);
    assert_eq!(parent_id_name.stats_offset, 55 + 4);
}

#[test]
fn hostile_leaf_pages_are_refused_precisely() {
    let mut wrong_type = page(7).to_vec();
    wrong_type[0] = 0x02;
    let error = decode_leaf(7, &wrong_type).expect_err("data page is not a leaf");
    assert!(matches!(error, StorageError::Corrupt { .. }), "{error:?}");

    let mut truncated = page(7).to_vec();
    truncated.truncate(0x100);
    assert!(matches!(
        decode_leaf(7, &truncated),
        Err(StorageError::Corrupt { .. })
    ));

    // A bitmap set bit beyond the entry area cannot describe an entry.
    let mut outside = page(7).to_vec();
    outside[0x1B + 452] |= 0x80;
    assert!(matches!(
        decode_leaf(7, &outside),
        Err(StorageError::Corrupt { .. })
    ));
}

#[test]
fn an_overfull_leaf_is_refused_instead_of_splitting() {
    let mut leaf = decode_leaf(12, page(12)).expect("empty leaf decodes");
    assert!(leaf.entries.is_empty(), "page 12 is an empty leaf");
    // Fill the entry area past the page end: 4 KiB of payload cannot fit.
    leaf.entries.push(vec![0x7F; PAGE_SIZE]);
    let error = encode_leaf(&leaf, PAGE_SIZE).expect_err("no splitting");
    match error {
        StorageError::Unsupported { reason } => assert!(reason.contains("leaf"), "{reason}"),
        other => panic!("expected Unsupported, found {other:?}"),
    }
}

/// The decoded leaf of the template's catalog must reflect its parent.
#[test]
fn decoded_leaves_report_their_parent_tdef() {
    let leaf: LeafPage = decode_leaf(10, page(10)).expect("MSysACEs leaf");
    assert_eq!(leaf.parent, 3);
    assert_eq!(leaf.entries.len(), 16);
    let index: PhysicalIndex = physical_indexes(&catalog()).expect("indexes")[0].clone();
    assert!(!index.columns.is_empty());
}
