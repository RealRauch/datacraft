//! Row count without a read (phase 101, step 101.5; item M132): the Jet
//! adapter answers from the TDEF's own counter, and the number agrees with
//! the rows a full read returns.

use datacraft_core::port::StoragePort;
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn the_tdef_counter_matches_the_rows_of_a_full_read() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2000.mdb"), None).expect("opens");
    let tables = port.tables();
    assert!(!tables.is_empty());
    for table in &tables {
        let counted = port.row_count(&table.name).expect("a count");
        let rows = port
            .read_rows(
                &table.name,
                datacraft_core::port::RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .expect("a read");
        assert_eq!(
            counted,
            rows.len(),
            "the counter agrees with the read for {:?}",
            table.name
        );
    }
}

#[test]
fn an_unknown_table_is_refused() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2000.mdb"), None).expect("opens");
    assert!(port.row_count("Missing").is_err());
}
