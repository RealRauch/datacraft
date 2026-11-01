//! VBA extraction contract suite (step 17.2, item L5).
//!
//! The pinned Northwind 2007 fixture carries a real 18-module VBA project
//! in `MSysAccessStorage` (VBA/VBAProject/VBA). The extraction is
//! deterministic: `dir` records define name, stream name, offset and kind;
//! each module stream is [performance cache][compressed source].

use std::path::{Path, PathBuf};

use datacraft_storage::jet::{Jet4TableReader, StoredModuleKind};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn reader(name: &str) -> Jet4TableReader {
    Jet4TableReader::new(std::fs::read(fixture(name)).expect("fixture is readable"))
        .expect("fixture parses")
}

#[test]
fn northwind_vba_project_decodes_every_module_with_its_source() {
    let project = reader("northwind2007.accdb")
        .vba_project()
        .expect("VBA project decodes")
        .expect("northwind has a VBA project");
    assert_eq!(project.modules.len(), 18, "{:#?}", project.modules);
    let names: Vec<&str> = project
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(
        &names[..8],
        &[
            "Utilities",
            "ErrorHandling",
            "CustomerOrders",
            "Inventory",
            "Privileges",
            "DomainFunctionWrappers",
            "RecordsetWrapper",
            "PurchaseOrders",
        ]
    );

    let utilities = &project.modules[0];
    assert_eq!(utilities.kind, StoredModuleKind::Standard);
    assert!(
        utilities
            .source
            .starts_with("Attribute VB_Name = \"Utilities\""),
        "{}",
        utilities.source
    );
    assert!(utilities.source.contains("Option Compare Database"));
    assert!(utilities.source.contains("Public Enum StringIDEnum"));

    let form = project
        .modules
        .iter()
        .find(|module| module.name == "Form_Sales Reports Dialog")
        .expect("form module");
    assert_eq!(form.kind, StoredModuleKind::Form);
    assert!(
        form.source
            .starts_with("Attribute VB_Name = \"Form_Sales Reports Dialog\"")
    );

    let report = project
        .modules
        .iter()
        .find(|module| module.name == "Report_Yearly Sales Report")
        .expect("report module");
    assert_eq!(report.kind, StoredModuleKind::Report);

    let class = project
        .modules
        .iter()
        .find(|module| module.name == "ErrorHandling")
        .expect("class module");
    assert_eq!(class.kind, StoredModuleKind::Class);
}

#[test]
fn the_ace_template_project_exists_but_has_no_modules() {
    let project = reader("testV2007.accdb")
        .vba_project()
        .expect("template project decodes")
        .expect("the template carries an empty VBA project");
    assert!(project.modules.is_empty());
}

#[test]
fn jet3_vba_projects_are_not_decoded_yet() {
    assert!(
        reader("testV1997.mdb")
            .vba_project()
            .expect("Jet3 inventory is not an error")
            .is_none()
    );
}
