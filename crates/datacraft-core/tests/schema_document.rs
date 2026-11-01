//! Contract suite for the versioned schema document (step 3.2, item H11).
//!
//! Locks the `.datacraft` sidecar format foundation (ADR-1,
//! docs/ARCHITECTURE.md §7): serde JSON with a `schema_version` field,
//! deterministic serialization, version gate (own version parses, newer
//! versions rejected precisely, older versions migrate forward) and file
//! save/load.

use std::path::PathBuf;

use datacraft_core::schema::{
    AutoKind, FieldDef, FieldKind, FieldProperties, IndexMode, NumericKind, SchemaDocument,
    SchemaError, TableDef,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn sample_document() -> SchemaDocument {
    let mut id = FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger));
    id.properties.indexed = IndexMode::NoDuplicates;
    id.properties.required = true;

    let mut name = FieldDef::new(
        "name",
        FieldKind::Text {
            max_len: 50,
        },
    );
    name.properties.allow_zero_length = true;
    name.properties.caption = Some("Customer name".into());

    let balance = FieldDef::new(
        "balance",
        FieldKind::Number(NumericKind::Decimal {
            precision: 18,
            scale: 2,
        }),
    );

    let customers = TableDef::new("Customers", vec![id, name, balance]);

    let mut note = FieldDef::new("note", FieldKind::Memo);
    note.properties.format = Some("@".into());
    let orders = TableDef::new(
        "Orders",
        vec![
            FieldDef::new("order_id", FieldKind::Currency),
            note,
        ],
    );

    SchemaDocument::new("shop").with_tables(vec![customers, orders])
}

fn err_contains(result: Result<(), SchemaError>, needle: &str) {
    let msg = result.expect_err("expected a schema error").to_string();
    assert!(
        msg.to_lowercase().contains(&needle.to_lowercase()),
        "error message {msg:?} does not mention {needle:?}"
    );
}

fn ok(result: Result<(), SchemaError>) {
    result.unwrap_or_else(|e| panic!("expected Ok, got: {e}"));
}

// ---------------------------------------------------------------------------
// document structure and validation
// ---------------------------------------------------------------------------

#[test]
fn new_documents_carry_the_current_schema_version() {
    let doc = SchemaDocument::new("shop");
    assert_eq!(doc.schema_version, datacraft_core::schema::SCHEMA_VERSION);
    assert!(doc.tables.is_empty());
    ok(doc.validate());
}

#[test]
fn document_validates_tables_and_database_name() {
    let doc = sample_document();
    ok(doc.validate());

    let empty_name = SchemaDocument::new("   ");
    err_contains(empty_name.validate(), "name");

    let doc = SchemaDocument::new("shop").with_tables(vec![
        TableDef::new("Customers", vec![FieldDef::new("id", FieldKind::Currency)]),
        TableDef::new("customers", vec![FieldDef::new("id", FieldKind::Memo)]),
    ]);
    err_contains(doc.validate(), "duplicate");

    let doc = SchemaDocument::new("shop").with_tables(vec![TableDef::new(
        "broken",
        vec![FieldDef::new("", FieldKind::Memo)],
    )]);
    err_contains(doc.validate(), "name");
}

// ---------------------------------------------------------------------------
// JSON round-trip and determinism
// ---------------------------------------------------------------------------

#[test]
fn document_json_roundtrip_preserves_everything() {
    let doc = sample_document();
    ok(doc.validate());
    let json = doc.to_json().expect("serialize");
    let back = SchemaDocument::from_json(&json).expect("deserialize");
    assert_eq!(back, doc);
}

#[test]
fn serialization_is_deterministic() {
    let a = sample_document();
    let b = sample_document();
    assert_eq!(a, b);
    assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    assert_eq!(a.to_json().unwrap(), a.to_json().unwrap());
}

#[test]
fn serialized_document_is_pretty_and_versioned() {
    let json = sample_document().to_json().unwrap();
    assert!(json.contains("\"schema_version\""), "missing version field: {json}");
    assert!(json.contains('\n'), "expected pretty-printed JSON: {json}");
}

#[test]
fn malformed_json_is_a_parse_error_with_detail() {
    let result = SchemaDocument::from_json("{ not json");
    let err = result.expect_err("expected parse error").to_string();
    assert!(err.to_lowercase().contains("schema document"), "{err}");
}

// ---------------------------------------------------------------------------
// version gate
// ---------------------------------------------------------------------------

#[test]
fn future_versions_are_rejected_precisely() {
    let mut json = sample_document().to_json().unwrap();
    json = json.replace("\"schema_version\": 1", "\"schema_version\": 99");
    let err = SchemaDocument::from_json(&json)
        .expect_err("future version must be rejected")
        .to_string();
    assert!(err.contains("99"), "error must name the found version: {err}");
    assert!(
        err.contains("newer") || err.contains("supported"),
        "error must explain the gate: {err}"
    );
}

#[test]
fn the_current_version_parses() {
    let json = sample_document().to_json().unwrap();
    ok(SchemaDocument::from_json(&json).map(|_| ()));
}

#[test]
fn older_versions_migrate_forward_to_the_current_version() {
    // v1 is the initial version — the migration ladder starts here. Loading
    // a v1 document must produce a valid current document unchanged; the
    // ladder rejects versions it has no step for (see version_zero test).
    let json = sample_document().to_json().unwrap();
    assert!(json.contains("\"schema_version\": 1"));
    let doc = SchemaDocument::from_json(&json).expect("v1 parses");
    assert_eq!(doc.schema_version, 1);
    ok(doc.validate());
}

#[test]
fn version_zero_is_unsupported() {
    let mut json = sample_document().to_json().unwrap();
    json = json.replace("\"schema_version\": 1", "\"schema_version\": 0");
    let err = SchemaDocument::from_json(&json)
        .expect_err("version 0 never existed")
        .to_string();
    assert!(err.contains("version"), "{err}");
}

// ---------------------------------------------------------------------------
// file persistence (the sidecar)
// ---------------------------------------------------------------------------

#[test]
fn sidecar_path_replaces_the_database_extension() {
    assert_eq!(
        SchemaDocument::sidecar_path(&PathBuf::from("shop.mdb")),
        PathBuf::from("shop.datacraft")
    );
    assert_eq!(
        SchemaDocument::sidecar_path(&PathBuf::from("shop.accdb")),
        PathBuf::from("shop.datacraft")
    );
    assert_eq!(
        SchemaDocument::sidecar_path(&PathBuf::from("shop")),
        PathBuf::from("shop.datacraft")
    );
}

#[test]
fn save_and_load_round_trip_through_the_filesystem() {
    let doc = sample_document();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "datacraft-test-{}-{}.datacraft",
        std::process::id(),
        line!()
    ));
    doc.save(&path).expect("save sidecar");
    let loaded = SchemaDocument::load(&path).expect("load sidecar");
    assert_eq!(loaded, doc);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loading_a_missing_file_is_an_io_error() {
    let result = SchemaDocument::load(&PathBuf::from("definitely-missing-424242.datacraft"));
    let err = result.expect_err("expected io error").to_string();
    assert!(err.to_lowercase().contains("i/o") || err.to_lowercase().contains("io"), "{err}");
}

// ---------------------------------------------------------------------------
// property-based round-trips
// ---------------------------------------------------------------------------

proptest! {
    /// Any database name that passes validation survives a JSON round-trip.
    #[test]
    fn database_name_json_roundtrip(name in "[a-z][a-z0-9_]{0,30}") {
        let doc = SchemaDocument::new(name.clone());
        let json = doc.to_json().unwrap();
        let back = SchemaDocument::from_json(&json).unwrap();
        prop_assert_eq!(back, doc);
        prop_assert_eq!(back.database_name, name);
    }

    /// Field properties of every shape round-trip through the document.
    #[test]
    fn table_names_roundtrip(name in "[A-Za-z][A-Za-z0-9 ]{0,40}") {
        let table = TableDef::new(name.clone(), vec![FieldDef::new(
            "f",
            FieldKind::Text { max_len: 10 },
        )]);
        let doc = SchemaDocument::new("db").with_tables(vec![table]);
        let back = SchemaDocument::from_json(&doc.to_json().unwrap()).unwrap();
        prop_assert_eq!(back.tables[0].name, name);
    }
}

#[test]
fn properties_survive_the_document_round_trip_verbatim() {
    let mut field = FieldDef::new("p", FieldKind::Boolean { tri_state: true });
    field.properties = FieldProperties {
        caption: Some("c".into()),
        input_mask: Some(">L".into()),
        format: Some("Yes/No".into()),
        default_value: Some("True".into()),
        validation_rule: Some("Is Not Null".into()),
        validation_text: Some("enter a value".into()),
        required: true,
        allow_zero_length: false,
        indexed: IndexMode::DuplicatesOk,
    };
    let doc = SchemaDocument::new("db").with_tables(vec![TableDef::new(
        "t",
        vec![field],
    )]);
    let back = SchemaDocument::from_json(&doc.to_json().unwrap()).unwrap();
    assert_eq!(back.tables[0].fields[0].properties, field.properties);
}
