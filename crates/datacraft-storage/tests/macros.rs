//! Macro inventory contract suite (step 17.1, item L4).
//!
//! The pinned Northwind 2007 fixture is the only committed database with
//! named macros; the jackcess templates carry empty `Scripts` containers.
//! Assertions pin the catalog names and the readable definition arguments
//! (the coarse action list — no execution semantics).

use std::path::{Path, PathBuf};

use datacraft_storage::jet::{AccessStorage, Jet4TableReader};

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
fn catalog_lists_the_northwind_macros_in_storage_order() {
    let reader = reader("northwind2007.accdb");
    assert_eq!(
        reader.macro_names().expect("catalog decodes"),
        vec!["AutoExec", "Delete All Data"]
    );
}

#[test]
fn northwind_macros_decode_their_readable_definition_arguments() {
    let reader = reader("northwind2007.accdb");
    let macros = reader.macros().expect("macro definitions decode");
    assert_eq!(macros.len(), 2, "{macros:#?}");

    let autoexec = &macros[0];
    assert_eq!(autoexec.name, "AutoExec");
    assert!(autoexec.definition_decoded);
    assert_eq!(
        autoexec.arguments,
        vec![
            "Northwind Traders",
            "Not [CurrentProject].[IsTrusted]",
            "Startup Screen",
            "[CurrentProject].[IsTrusted]",
            "Login Dialog",
        ]
    );

    let delete = &macros[1];
    assert_eq!(delete.name, "Delete All Data");
    assert!(delete.definition_decoded);
    assert_eq!(
        delete.arguments,
        vec![
            "MsgBox(\"Do you really want to delete all data in the database?\",260)",
            "Delete * from [Invoices]",
            "Delete * from [Order Details]",
            "Delete * from [Orders]",
            "Delete * from [Purchase Order Details]",
            "Delete * from [Purchase Orders]",
            "Delete * from [Inventory Transactions]",
        ]
    );
}

#[test]
fn the_access_storage_tree_exposes_the_scripts_container_and_its_blobs() {
    let reader = reader("northwind2007.accdb");
    let storage = AccessStorage::read(&reader)
        .expect("storage decodes")
        .expect("northwind has MSysAccessStorage");
    let scripts = storage
        .entry("Scripts")
        .expect("Scripts container is present");
    assert_eq!(scripts.entry_type, AccessStorage::TYPE_STORAGE);
    let children = storage.children(scripts.id);
    let blobs: Vec<&str> = children
        .iter()
        .filter(|child| child.is_stream())
        .map(|child| child.name.as_str())
        .collect();
    assert!(blobs.contains(&"Blob"), "script blobs found: {blobs:?}");
    assert!(
        children.iter().any(|child| child.name.ends_with("DirData")),
        "script directory is present: {children:#?}"
    );
}

#[test]
fn ace_template_without_macros_reports_an_empty_inventory() {
    let reader = reader("testV2007.accdb");
    assert!(reader.macro_names().expect("catalog decodes").is_empty());
    assert!(reader.macros().expect("definitions decode").is_empty());
}

#[test]
fn jet3_catalog_macro_names_are_decoded_without_definition_bytes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV1997.mdb");
    // The Jet3 template has no macros either; the type constant is shared
    // with ACE (-32766), so the catalog path is the same.
    let reader = Jet4TableReader::new(std::fs::read(path).expect("fixture is readable"))
        .expect("fixture parses");
    assert!(reader.macro_names().expect("catalog decodes").is_empty());
    assert!(reader.macros().expect("definitions decode").is_empty());
}
