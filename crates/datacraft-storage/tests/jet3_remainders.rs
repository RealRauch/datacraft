//! Jet3 remainders (step 16.3, item L3): chained table definitions, overflow
//! rows and the >255-byte jump table.
//!
//! Fixture: `fixtures/testV1997.mdb` (jackcess V1997, pristine). The file
//! carries a chained TDEF (page 34 → 44) and overflow rows that the old
//! reader could not follow on the Jet3 path.

use std::path::{Path, PathBuf};

use datacraft_core::port::{RowRange, StoragePort};
use datacraft_jet::StorageValue;
use datacraft_jet::jet::{Jet4TableReader, JetTableDef};
use datacraft_storage::detecting::DetectingPort;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV1997.mdb")
}

fn jump_table_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("test2V1997.mdb")
}

#[test]
fn the_pinned_jet3_fixture_enumerates_and_reads_every_table() {
    let mut port = DetectingPort::new();
    port.open(&fixture(), None).expect("the Jet3 fixture opens");
    let tables = port.tables();
    assert!(!tables.is_empty(), "the fixture has user tables");
    for table in tables {
        let rows = port
            .read_rows(
                &table.name,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .unwrap_or_else(|e| panic!("read {}: {e}", table.name));
        eprintln!("table {}: {} rows", table.name, rows.len());
    }
}

#[test]
fn chained_jet3_tdefs_decode_across_pages() {
    let file = std::fs::read(fixture()).expect("fixture");
    let def = JetTableDef::decode(&file, 34).expect("the chained TDEF decodes");
    assert!(
        def.columns.len() > 50,
        "the chain yields the full wide schema ({} columns)",
        def.columns.len()
    );
}

#[test]
fn the_pinned_jet3_fixture_decodes_its_290_byte_jump_table_row() {
    let file = std::fs::read(jump_table_fixture()).expect("test2 fixture");
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader
        .read_table(29)
        .expect("the 290-byte jump-table row decodes");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].len(), 74, "all 74 columns decode");
    assert!(
        table.rows[0]
            .iter()
            .any(|value| matches!(value, StorageValue::Text(text) if !text.is_empty())),
        "the row carries text values"
    );
}

#[test]
fn jet3_overflow_rows_resolve_through_the_format_specific_decoder() {
    // The catalog itself contains overflow rows; enumerating it proves the
    // overflow pointer is decoded with the Jet3 10-byte page header, not the
    // Jet4 layout.
    let file = std::fs::read(fixture()).expect("fixture");
    let reader = datacraft_jet::jet::Jet4TableReader::new(file).expect("reader");
    let entries = reader.catalog().expect("catalog with overflow rows");
    assert!(
        entries.iter().any(|entry| entry.name == "MSysObjects"),
        "catalog entries decode"
    );
}
