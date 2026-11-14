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

fn generated_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formDesignsV2016.accdb")
        .canonicalize()
        .expect("the generated fixture is present")
}

#[test]
fn generated_fixture_maps_the_six_control_kinds() {
    let expected = SchemaDocument::from_json(include_str!("data/form-designs.datacraft.json"))
        .expect("the expected sidecar is valid");
    let imported = import_database(&generated_fixture()).expect("the generated fixture imports");
    assert_eq!(imported.document, expected);

    let all = imported
        .report
        .objects
        .iter()
        .find(|object| object.object == "All Controls")
        .expect("the six-kind form");
    assert_eq!(all.imported_controls, 6, "all six control kinds map");
    assert!(all.unsupported_controls.is_empty());
    assert_eq!(
        all.unsupported_properties.len(),
        1,
        "only the combo row source is unsupported"
    );
    assert_eq!(all.unsupported_properties[0].property, "RowSource");

    let details = imported
        .report
        .objects
        .iter()
        .find(|object| object.object == "Category Details")
        .expect("the subform source form");
    assert_eq!(details.imported_controls, 0);
    assert!(details.unsupported_properties.is_empty());
}

#[test]
fn generated_defaults_are_reported_instead_of_guessed() {
    let imported = import_database(&generated_fixture()).expect("the generated fixture imports");
    let defaults = imported
        .report
        .objects
        .iter()
        .find(|object| object.object == "Defaults")
        .expect("the defaults form");
    assert_eq!(defaults.imported_controls, 0);
    assert_eq!(
        defaults.unsupported_properties.len(),
        11,
        "{:?}",
        defaults.unsupported_properties
    );
    for entry in &defaults.unsupported_properties {
        assert!(
            entry
                .reason
                .contains("no value and no confirmed Access default"),
            "{entry:?}"
        );
    }
    assert!(
        defaults.unsupported_controls.is_empty(),
        "the control kinds are known; only the omitted geometry is unsupported"
    );
}
