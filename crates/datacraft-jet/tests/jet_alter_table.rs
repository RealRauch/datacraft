//! Design-time schema editing on the Jet4 writer (step 24.3, item M9).
//!
//! Add/drop/rename and the `required` property edit the TDEF and rewrite
//! existing rows; row data survives, the file reopens consistently and a
//! rejected change leaves the file untouched.

use std::path::{Path, PathBuf};

use datacraft_core::port::{PortError, RowRange, SchemaChange, StoragePort};
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, FieldValue, NumericKind, TableDef,
};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

fn fixture_copy(stem: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("datacraft-alter-{}-{stem}.mdb", std::process::id()));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

fn base_table() -> TableDef {
    TableDef::new(
        "AlterMe",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("note", FieldKind::Text { max_len: 20 }),
        ],
    )
}

fn opened(stem: &str) -> (Jet4Port, PathBuf) {
    let path = fixture_copy(stem);
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    port.create_table(&base_table()).expect("create");
    port.insert_row(
        "AlterMe",
        &[FieldValue::Integer(1), FieldValue::Text("first".to_owned())],
    )
    .expect("insert");
    (port, path)
}

fn rows(port: &Jet4Port) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        "AlterMe",
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("rows read")
}

#[test]
fn adding_a_field_keeps_rows_and_fills_null() {
    let (mut port, path) = opened("add");
    port.alter_table(
        "AlterMe",
        SchemaChange::AddField {
            field: FieldDef::new("extra", FieldKind::Text { max_len: 10 }),
        },
    )
    .expect("field added");
    assert_eq!(
        rows(&port),
        [vec![
            FieldValue::Integer(1),
            FieldValue::Text("first".to_owned()),
            FieldValue::Null,
        ]]
    );
    port.close();

    port.open(&path, None).expect("reopen");
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "AlterMe")
        .expect("table survives");
    assert_eq!(definition.fields.len(), 3);
    assert_eq!(definition.fields[2].name, "extra");
    assert_eq!(rows(&port)[0][2], FieldValue::Null);
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn dropping_a_field_keeps_the_remaining_values() {
    let (mut port, path) = opened("drop");
    port.alter_table(
        "AlterMe",
        SchemaChange::DropField {
            field: "ID".to_owned(),
        },
    )
    .expect("field dropped (case-insensitive)");
    assert_eq!(rows(&port), [vec![FieldValue::Text("first".to_owned())]]);
    port.close();

    port.open(&path, None).expect("reopen");
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "AlterMe")
        .expect("table survives");
    assert_eq!(definition.fields.len(), 1);
    assert_eq!(definition.fields[0].name, "note");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn renaming_a_field_keeps_row_data() {
    let (mut port, path) = opened("rename");
    port.alter_table(
        "AlterMe",
        SchemaChange::RenameField {
            field: "note".to_owned(),
            new_name: "text".to_owned(),
        },
    )
    .expect("renamed");
    assert_eq!(rows(&port)[0][1], FieldValue::Text("first".to_owned()));
    port.close();

    port.open(&path, None).expect("reopen");
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "AlterMe")
        .expect("table survives");
    assert_eq!(definition.fields[1].name, "text");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_required_property_round_trips() {
    let (mut port, path) = opened("required");
    port.alter_table(
        "AlterMe",
        SchemaChange::SetFieldProperties {
            field: "note".to_owned(),
            properties: FieldProperties {
                required: true,
                ..Default::default()
            },
        },
    )
    .expect("required set");
    port.close();

    port.open(&path, None).expect("reopen");
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "AlterMe")
        .expect("table survives");
    assert!(definition.fields[1].properties.required);
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rejected_changes_leave_the_file_untouched() {
    let (mut port, path) = opened("reject");
    let before = std::fs::read(&path).expect("copy readable");
    let mut required = FieldDef::new("strict", FieldKind::Text { max_len: 5 });
    required.properties.required = true;
    let error = port
        .alter_table("AlterMe", SchemaChange::AddField { field: required })
        .expect_err("required fields need a default for existing rows");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    port.close();
    let after = std::fs::read(&path).expect("copy readable");
    assert_eq!(before, after, "rejected changes leave the file untouched");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_and_unknown_fields_are_rejected_precisely() {
    let (mut port, path) = opened("duplicate");
    let duplicate = port
        .alter_table(
            "AlterMe",
            SchemaChange::AddField {
                field: FieldDef::new("NOTE", FieldKind::Text { max_len: 5 }),
            },
        )
        .expect_err("duplicate names are rejected");
    assert!(duplicate.to_string().contains("duplicate"), "{duplicate}");
    let unknown = port
        .alter_table(
            "AlterMe",
            SchemaChange::RenameField {
                field: "missing".to_owned(),
                new_name: "x".to_owned(),
            },
        )
        .expect_err("unknown fields are rejected");
    assert!(unknown.to_string().contains("not found"), "{unknown}");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn unsupported_shapes_and_last_fields_are_rejected() {
    let (mut port, path) = opened("shapes");
    let error = port
        .alter_table(
            "AlterMe",
            SchemaChange::AddField {
                field: FieldDef::new(
                    "blob",
                    FieldKind::Binary(datacraft_core::schema::BinaryKind::Attachment),
                ),
            },
        )
        .expect_err("complex columns are read-only");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    port.alter_table(
        "AlterMe",
        SchemaChange::DropField {
            field: "id".to_owned(),
        },
    )
    .expect("first drop");
    let error = port
        .alter_table(
            "AlterMe",
            SchemaChange::DropField {
                field: "note".to_owned(),
            },
        )
        .expect_err("the last field cannot be dropped");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_renames_and_property_clears_are_handled() {
    let (mut port, path) = opened("renames");
    let error = port
        .alter_table(
            "AlterMe",
            SchemaChange::RenameField {
                field: "note".to_owned(),
                new_name: "ID".to_owned(),
            },
        )
        .expect_err("duplicate rename");
    assert!(error.to_string().contains("duplicate"), "{error}");
    port.alter_table(
        "AlterMe",
        SchemaChange::SetFieldProperties {
            field: "note".to_owned(),
            properties: FieldProperties {
                required: true,
                ..Default::default()
            },
        },
    )
    .expect("set required");
    port.alter_table(
        "AlterMe",
        SchemaChange::SetFieldProperties {
            field: "note".to_owned(),
            properties: FieldProperties::default(),
        },
    )
    .expect("clear required");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_change_that_does_not_fit_the_tdef_page_is_rejected() {
    let (mut port, path) = opened("overflow");
    let mut rejected = false;
    for index in 0..80 {
        let field = FieldDef::new(
            format!("field_{index:03}_{}", "x".repeat(60)),
            FieldKind::Text { max_len: 60 },
        );
        match port.alter_table("AlterMe", SchemaChange::AddField { field }) {
            Ok(()) => {}
            Err(error) => {
                assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
                assert!(error.to_string().contains("does not fit"), "{error}");
                rejected = true;
                break;
            }
        }
    }
    assert!(rejected, "the TDEF page must eventually overflow");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn property_sheets_are_accepted_and_only_required_is_physical() {
    let (mut port, path) = opened("properties");
    let properties = FieldProperties {
        caption: Some("Note".to_owned()),
        required: true,
        ..Default::default()
    };
    port.alter_table(
        "AlterMe",
        SchemaChange::SetFieldProperties {
            field: "note".to_owned(),
            properties,
        },
    )
    .expect("the property sheet is accepted");
    port.close();

    port.open(&path, None).expect("reopen");
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "AlterMe")
        .expect("table survives");
    assert!(definition.fields[1].properties.required);
    assert!(
        definition.fields[1].properties.caption.is_none(),
        "the caption is sidecar authority, not a TDEF field"
    );
    port.close();
    let _ = std::fs::remove_file(&path);
}
