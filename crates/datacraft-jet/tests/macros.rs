//! Macro inventory contract suite (step 17.1, item L4; fixture replaced by
//! H50 with the self-created programmability fixture).
//!
//! The pinned `programmability.accdb` carries two named macros with
//! readable, non-destructive arguments; the jackcess templates carry empty
//! `Scripts` containers. Assertions pin the catalog names and the readable
//! definition arguments (the coarse action list — no execution semantics).

use std::path::{Path, PathBuf};

use datacraft_jet::jet::{AccessStorage, Jet4TableReader};

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
fn catalog_lists_the_fixture_macros_in_storage_order() {
    let reader = reader("programmability.accdb");
    assert_eq!(
        reader.macro_names().expect("catalog decodes"),
        vec!["Open Item Browser", "Refresh Stock Views"]
    );
}

#[test]
fn fixture_macros_decode_their_readable_definition_arguments() {
    let reader = reader("programmability.accdb");
    let macros = reader.macros().expect("macro definitions decode");
    assert_eq!(macros.len(), 2, "{macros:#?}");

    let open = &macros[0];
    assert_eq!(open.name, "Open Item Browser");
    assert!(open.definition_decoded);
    assert_eq!(
        open.arguments,
        vec![
            "Inventory sample ready.",
            "Items Browser",
            "ItemName",
            "Pick a category to filter the view.",
            "Category Picker",
        ]
    );

    let refresh = &macros[1];
    assert_eq!(refresh.name, "Refresh Stock Views");
    assert!(refresh.definition_decoded);
    assert_eq!(
        refresh.arguments,
        vec![
            "Items In Stock",
            "ItemName",
            "[StockLevel] > 0",
            "Stock By Category",
        ]
    );
}

#[test]
fn the_access_storage_tree_exposes_the_scripts_container_and_its_blobs() {
    let reader = reader("programmability.accdb");
    let storage = AccessStorage::read(&reader)
        .expect("storage decodes")
        .expect("the fixture has MSysAccessStorage");
    let scripts = storage
        .entry("Scripts")
        .expect("Scripts container is present");
    assert_eq!(scripts.entry_type, AccessStorage::TYPE_STORAGE);
    let children = storage.children(scripts.id);
    let blobs: Vec<&str> = children
        .iter()
        .flat_map(|child| storage.children(child.id))
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
