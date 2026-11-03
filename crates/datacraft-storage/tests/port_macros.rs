//! Port-level macro inventory contracts (step 17.1, item L4).
//!
//! The inventory crosses the `StoragePort` seam so every client (UI, CLI,
//! MCP) sees the same names and coarse arguments — no storage types leak
//! into the core.

use std::path::{Path, PathBuf};

use datacraft_core::port::StoragePort;
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn jet_port_maps_the_northwind_macro_inventory() {
    let mut port = Jet4Port::new();
    port.open(&fixture("northwind2007.accdb"), None)
        .expect("fixture opens");
    let macros = port.macros().expect("inventory reads");
    assert_eq!(macros.len(), 2, "{macros:#?}");
    assert_eq!(macros[0].name, "AutoExec");
    assert!(macros[0].definition_decoded);
    assert_eq!(macros[0].actions.len(), 5);
    assert_eq!(macros[1].actions[1].argument, "Delete * from [Invoices]");
    port.close();
    assert!(port.macros().expect("closed port answers").is_empty());
}

#[test]
fn detecting_port_delegates_the_macro_inventory() {
    let mut port = DetectingPort::new();
    port.open(&fixture("northwind2007.accdb"), None)
        .expect("fixture opens");
    let names: Vec<String> = port
        .macros()
        .expect("inventory reads")
        .into_iter()
        .map(|macro_def| macro_def.name)
        .collect();
    assert_eq!(names, vec!["AutoExec", "Delete All Data"]);
}

#[test]
fn an_ace_template_without_macros_answers_an_empty_inventory() {
    let mut port = DetectingPort::new();
    port.open(&fixture("testV2007.accdb"), None)
        .expect("fixture opens");
    assert!(port.macros().expect("inventory reads").is_empty());
}
