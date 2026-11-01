//! Port-level VBA project contracts (step 17.2, item L5).

use std::path::{Path, PathBuf};

use datacraft_core::port::StoragePort;
use datacraft_storage::detecting::DetectingPort;
use datacraft_storage::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn jet_port_maps_the_northwind_vba_project() {
    let mut port = Jet4Port::new();
    port.open(&fixture("northwind2007.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project reads")
        .expect("northwind has a project");
    assert_eq!(project.modules.len(), 18);
    assert_eq!(project.modules[0].name, "Utilities");
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
    port.open(&fixture("northwind2007.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project reads")
        .expect("project present");
    assert_eq!(project.modules.len(), 18);
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
