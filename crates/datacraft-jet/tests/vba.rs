//! VBA extraction contract suite (step 17.2, item L5; fixture replaced by
//! H50 with the self-created programmability fixture).
//!
//! The pinned `programmability.accdb` carries a real 8-module VBA project in
//! `MSysAccessStorage` (VBA/VBAProject/VBA). The extraction is
//! deterministic: `dir` records define name, stream name, offset and kind;
//! each module stream is [performance cache][compressed source].

use std::path::{Path, PathBuf};

use datacraft_jet::jet::{Jet4TableReader, StoredModuleKind};

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
fn programmability_vba_project_decodes_every_module_with_its_source() {
    let project = reader("programmability.accdb")
        .vba_project()
        .expect("VBA project decodes")
        .expect("the fixture has a VBA project");
    assert_eq!(project.modules.len(), 8, "{:#?}", project.modules);
    let names: Vec<&str> = project
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(
        &names[..],
        &[
            "InventoryHelpers",
            "QueryHelpers",
            "ArchiveExporter",
            "TaskRunner",
            "AuditRecorder",
            "Form_Items Browser",
            "Form_Movement Editor",
            "Report_Stock Summary",
        ]
    );

    let standard = &project.modules[0];
    assert_eq!(standard.kind, StoredModuleKind::Standard);
    assert!(
        standard
            .source
            .starts_with("Attribute VB_Name = \"InventoryHelpers\""),
        "{}",
        standard.source
    );
    assert!(standard.source.contains("Option Compare Database"));
    assert!(standard.source.contains("Public Enum StockScopeEnum"));
    // Copy-token fodder: the repeated long identifier survives decompression.
    assert!(
        standard
            .source
            .contains("WarehouseInventoryLedgerEntryCaption")
    );
    // Non-ASCII content survives the Windows-1252 project code page.
    assert!(
        standard
            .source
            .contains("Lagerstraße Süd 12, 70182 Stuttgart")
    );

    let form = project
        .modules
        .iter()
        .find(|module| module.name == "Form_Items Browser")
        .expect("form module");
    assert_eq!(form.kind, StoredModuleKind::Form);
    assert!(
        form.source
            .starts_with("Attribute VB_Name = \"Form_Items Browser\"")
    );

    let report = project
        .modules
        .iter()
        .find(|module| module.name == "Report_Stock Summary")
        .expect("report module");
    assert_eq!(report.kind, StoredModuleKind::Report);

    let class = project
        .modules
        .iter()
        .find(|module| module.name == "TaskRunner")
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
