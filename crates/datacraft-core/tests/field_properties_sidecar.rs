//! Field-property persistence through the `.datacraft` sidecar (step 32.1,
//! item H45; ADR-1 addendum).
//!
//! The sidecar is DataCraft's authority for the property sheet; the Jet
//! file keeps its structure and stays Access-readable. Every modeled
//! property must survive a save/load or a sibling auto-load, and stale or
//! invalid overlays must fail precisely instead of losing data silently.

use std::path::{Path, PathBuf};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, IndexMode, NumericKind, SchemaDocument, TableDef,
};

fn dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "datacraft-properties-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn properties() -> FieldProperties {
    FieldProperties {
        caption: Some("Customer name".into()),
        input_mask: Some(">L<????????????????????".into()),
        format: Some("@".into()),
        default_value: Some("\"new\"".into()),
        validation_rule: Some("Len([name]) > 0".into()),
        validation_text: Some("A name is required".into()),
        calculated: None,
        required: true,
        allow_zero_length: false,
        indexed: IndexMode::DuplicatesOk,
    }
}

fn annotated_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef {
                name: "id".into(),
                kind: FieldKind::Number(NumericKind::LongInteger),
                properties: FieldProperties {
                    caption: Some("Identifier".into()),
                    required: true,
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
            FieldDef {
                name: "name".into(),
                kind: FieldKind::Text { max_len: 40 },
                properties: properties(),
            },
            FieldDef {
                name: "total".into(),
                kind: FieldKind::Number(NumericKind::Double),
                properties: FieldProperties {
                    calculated: Some("[id] * 2".into()),
                    format: Some("Fixed".into()),
                    ..FieldProperties::default()
                },
            },
        ],
    )
}

fn plain_table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("total", FieldKind::Number(NumericKind::Double)),
        ],
    )
}

fn core_with(table: TableDef, path: &Path) -> Core {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![table])));
    let events = core.dispatch(Command::OpenDatabase {
        path: path.to_path_buf(),
        password: None,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    core
}

fn failure(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected Failed: {events:?}"))
}

fn live_properties(core: &Core) -> Vec<FieldProperties> {
    core.inspect().database.expect("database open").tables[0]
        .fields
        .iter()
        .map(|field| field.properties.clone())
        .collect()
}

#[test]
fn the_sidecar_round_trips_every_modeled_property() {
    let dir = dir("roundtrip");
    let database = dir.join("shop.mdb");
    let sidecar = dir.join("props.datacraft");
    let annotated = annotated_table();

    let mut source = core_with(annotated.clone(), &database);
    let events = source.dispatch(Command::SaveDesignDocument {
        path: sidecar.clone(),
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let document = SchemaDocument::load(&sidecar).expect("sidecar decodes");
    assert_eq!(
        document.tables,
        vec![annotated],
        "the full property set is persisted"
    );

    let mut target = core_with(plain_table(), &database);
    let events = target.dispatch(Command::LoadDesignDocument { path: sidecar });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DesignDocumentLoaded { .. })),
        "{events:?}"
    );
    assert_eq!(
        live_properties(&target),
        live_properties(&source),
        "every property survives the load"
    );
}

#[test]
fn a_sibling_sidecar_applies_its_properties_on_open() {
    let dir = dir("auto");
    let database = dir.join("shop.mdb");
    let sidecar = database.with_extension("datacraft");
    let document = SchemaDocument::new("shop").with_tables(vec![annotated_table()]);
    document.save(&sidecar).expect("write the sidecar");

    let core = core_with(plain_table(), &database);
    assert_eq!(
        live_properties(&core),
        annotated_table()
            .fields
            .iter()
            .map(|field| field.properties.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        core.inspect()
            .database
            .expect("open")
            .sidecar_error
            .is_none(),
        "a matching sidecar loads cleanly"
    );
}

#[test]
fn a_stale_sidecar_table_is_a_precise_error() {
    let dir = dir("stale");
    let database = dir.join("shop.mdb");
    let sidecar = dir.join("stale.datacraft");
    let mut other = annotated_table();
    other.name = "Ghost".into();
    let document = SchemaDocument::new("shop").with_tables(vec![other]);
    document.save(&sidecar).expect("write the sidecar");

    let mut core = core_with(plain_table(), &database);
    let reason = failure(&core.dispatch(Command::LoadDesignDocument { path: sidecar }));
    assert!(reason.contains("Ghost"), "{reason}");
    assert_eq!(
        live_properties(&core),
        plain_table()
            .fields
            .iter()
            .map(|field| field.properties.clone())
            .collect::<Vec<_>>(),
        "a failed overlay changes nothing"
    );

    let sibling = database.with_extension("datacraft");
    document.save(&sibling).expect("write the sibling");
    let auto = core_with(plain_table(), &database);
    let error = auto
        .inspect()
        .database
        .expect("open")
        .sidecar_error
        .expect("the stale table is reported");
    assert!(error.contains("Ghost"), "{error}");
}

#[test]
fn an_invalid_property_encoding_is_refused() {
    let dir = dir("invalid");
    let database = dir.join("shop.mdb");
    let sidecar = dir.join("invalid.datacraft");
    let mut invalid = plain_table();
    invalid.fields[0].properties.allow_zero_length = true;
    let document = SchemaDocument::new("shop").with_tables(vec![invalid]);
    document.save(&sidecar).expect("write the sidecar");

    let mut core = core_with(plain_table(), &database);
    let reason = failure(&core.dispatch(Command::LoadDesignDocument { path: sidecar }));
    assert!(reason.contains("allow_zero_length"), "{reason}");

    let sibling = database.with_extension("datacraft");
    document.save(&sibling).expect("write the sibling");
    let auto = core_with(plain_table(), &database);
    let error = auto
        .inspect()
        .database
        .expect("open")
        .sidecar_error
        .expect("the invalid property is reported");
    assert!(error.contains("allow_zero_length"), "{error}");
}

#[test]
fn schema_edits_keep_the_other_properties() {
    let dir = dir("edits");
    let database = dir.join("shop.mdb");
    let sidecar = database.with_extension("datacraft");
    SchemaDocument::new("shop")
        .with_tables(vec![annotated_table()])
        .save(&sidecar)
        .expect("write the sidecar");
    let mut core = core_with(plain_table(), &database);

    let events = core.dispatch(Command::SetFieldProperties {
        table: "Customers".into(),
        field: "name".into(),
        properties: FieldProperties {
            caption: Some("Changed".into()),
            ..properties()
        },
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let props = live_properties(&core);
    assert_eq!(props[0].caption.as_deref(), Some("Identifier"));
    assert_eq!(props[1].caption.as_deref(), Some("Changed"));
    assert_eq!(props[2].calculated.as_deref(), Some("[id] * 2"));

    let events = core.dispatch(Command::AddField {
        table: "Customers".into(),
        field: FieldDef {
            name: "note".into(),
            kind: FieldKind::Text { max_len: 20 },
            properties: FieldProperties {
                caption: Some("Note".into()),
                ..FieldProperties::default()
            },
        },
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let props = live_properties(&core);
    assert_eq!(props[0].caption.as_deref(), Some("Identifier"));
    assert_eq!(props[2].calculated.as_deref(), Some("[id] * 2"));
    assert_eq!(props[3].caption.as_deref(), Some("Note"));
}
