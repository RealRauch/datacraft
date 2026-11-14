//! Contract test for the Route B mapping (step 78.1, item M88).
//!
//! The committed fixture plus the frozen expected sidecar are the contract:
//! the importer must reproduce every object, control and rect exactly and
//! report nothing unsupported for the supported subset.

use std::path::PathBuf;

use datacraft_core::schema::SchemaDocument;
use datacraft_import::import_database;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/programmability.accdb")
        .canonicalize()
        .expect("the committed fixture is present")
}

#[test]
fn committed_fixture_imports_to_the_expected_sidecar() {
    let expected = SchemaDocument::from_json(include_str!("data/programmability.datacraft.json"))
        .expect("the expected sidecar is valid");
    let imported = import_database(&fixture()).expect("the fixture imports");
    assert_eq!(imported.document, expected);
}

#[test]
fn the_report_names_every_object_and_skips_nothing() {
    let imported = import_database(&fixture()).expect("the fixture imports");
    let names: Vec<&str> = imported
        .report
        .objects
        .iter()
        .map(|object| object.object.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Category Picker",
            "Items Browser",
            "Movement Editor",
            "Stock Summary"
        ]
    );
    for object in &imported.report.objects {
        assert!(
            object.unsupported_controls.is_empty(),
            "{} has unsupported controls: {:?}",
            object.object,
            object.unsupported_controls
        );
        assert!(
            object.unsupported_properties.is_empty(),
            "{} has unsupported properties: {:?}",
            object.object,
            object.unsupported_properties
        );
    }
}

#[test]
fn the_fixture_is_never_written() {
    let path = fixture();
    let before = std::fs::read(&path).expect("read the fixture");
    let _ = import_database(&path).expect("the fixture imports");
    let after = std::fs::read(&path).expect("read the fixture again");
    assert_eq!(before, after, "importing must not modify the database");
}
