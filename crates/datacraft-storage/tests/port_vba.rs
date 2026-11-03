//! Port-level VBA project contracts (step 17.2, item L5; fixture replaced
//! by H50 with the self-created programmability fixture).

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
fn jet_port_maps_the_fixture_vba_project() {
    let mut port = Jet4Port::new();
    port.open(&fixture("programmability.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project reads")
        .expect("the fixture has a project");
    assert_eq!(project.modules.len(), 8);
    assert_eq!(project.modules[0].name, "InventoryHelpers");
    assert_eq!(
        project.modules[0].kind,
        datacraft_core::vba::VbaModuleKind::Standard
    );
    assert!(project.modules[0].source.contains("Attribute VB_Name"));
    port.close();
    assert!(port.vba_project().expect("closed port answers").is_none());
}

#[test]
fn detecting_port_delegates_the_vba_project() {
    let mut port = DetectingPort::new();
    port.open(&fixture("programmability.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project reads")
        .expect("project present");
    assert_eq!(project.modules.len(), 8);
}

#[test]
fn the_template_project_is_present_and_empty() {
    let mut port = DetectingPort::new();
    port.open(&fixture("testV2007.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project reads")
        .expect("template project present");
    assert!(project.modules.is_empty());
}
