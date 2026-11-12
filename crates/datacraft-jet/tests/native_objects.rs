//! Native Access form/report inventory (phase 74, step 74.1; item M87).
//!
//! The catalog's object types name the native forms (`-32768`) and reports
//! (`-32764`) — read-only, without decoding one definition byte. The
//! pinned `programmability.accdb` carries three forms and one report.

use datacraft_core::port::{NativeObjectKind, StoragePort};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn opened(name: &str) -> Jet4Port {
    let mut port = Jet4Port::new();
    port.open(&fixture(name), None).expect("fixture opens");
    port
}

#[test]
fn the_catalog_lists_native_forms_and_reports() {
    let port = opened("programmability.accdb");
    let objects = port.native_objects().expect("native object inventory");
    let forms: Vec<&str> = objects
        .iter()
        .filter(|object| object.kind == NativeObjectKind::Form)
        .map(|object| object.name.as_str())
        .collect();
    assert_eq!(
        forms,
        ["Category Picker", "Items Browser", "Movement Editor"],
        "{objects:?}"
    );
    let reports: Vec<&str> = objects
        .iter()
        .filter(|object| object.kind == NativeObjectKind::Report)
        .map(|object| object.name.as_str())
        .collect();
    assert_eq!(reports, ["Stock Summary"], "{objects:?}");
    // The inventory never turns a form into a table.
    let catalog = port.tables();
    let tables: Vec<&str> = catalog.iter().map(|table| table.name.as_str()).collect();
    assert!(!tables.contains(&"Items Browser"), "{tables:?}");
}

#[test]
fn a_database_without_native_objects_reports_an_empty_inventory() {
    let port = opened("testV2000.mdb");
    assert!(port.native_objects().expect("inventory").is_empty());
}
