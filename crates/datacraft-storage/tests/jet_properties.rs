//! Field-property persistence on a real Jet4 file (step 32.1, item H45).
//!
//! The Jet catalog keeps `NULL` LvProp (the file stays Access-readable);
//! the sibling `.datacraft` sidecar is the property authority. A create,
//! a schema edit and a reopen must preserve every modeled property, while
//! the database file itself remains readable through the plain Jet port.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, RowRange, StoragePort};
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, IndexMode, NumericKind, TableDef,
};
use datacraft_storage::port_adapter::Jet4Port;

fn scratch_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "datacraft-jet-properties-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn table() -> TableDef {
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
                properties: FieldProperties {
                    caption: Some("Customer name".into()),
                    default_value: Some("\"new\"".into()),
                    validation_rule: Some("Len([name]) > 0".into()),
                    validation_text: Some("A name is required".into()),
                    indexed: IndexMode::DuplicatesOk,
                    ..FieldProperties::default()
                },
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

fn database_and_sidecar(dir: &Path) -> (PathBuf, PathBuf) {
    let database = dir.join("shop.mdb");
    (database.clone(), database.with_extension("datacraft"))
}

fn open_core(database: &Path) -> Core {
    let mut core = Core::new(Box::new(Jet4Port::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: database.to_path_buf(),
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

fn properties(core: &Core) -> Vec<FieldProperties> {
    core.inspect().database.expect("open").tables[0]
        .fields
        .iter()
        .map(|field| field.properties.clone())
        .collect()
}

#[test]
fn properties_survive_create_edit_and_reopen_on_jet4() {
    let dir = scratch_dir();
    let (database, sidecar) = database_and_sidecar(&dir);

    let mut core = Core::new(Box::new(Jet4Port::new()));
    assert!(
        !core
            .dispatch(Command::CreateDatabase {
                path: database.clone(),
                format: DatabaseFormat::Jet4,
            })
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    );
    core = open_core(&database);
    let events = core.dispatch(Command::CreateTable {
        definition: table(),
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    assert!(
        !core
            .dispatch(Command::SaveDesignDocument {
                path: sidecar.clone(),
            })
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    );
    let original = properties(&core);
    core.dispatch(Command::CloseDatabase);

    // Reopen: the sibling sidecar overlays the properties on the physical
    // catalog without touching the file's structure.
    let mut core = open_core(&database);
    assert_eq!(properties(&core), original);
    assert!(
        core.inspect()
            .database
            .expect("open")
            .sidecar_error
            .is_none(),
        "the sidecar loads cleanly"
    );

    // A design-time edit refreshes the catalog from the port; the other
    // fields' properties must survive the refresh.
    let events = core.dispatch(Command::SetFieldProperties {
        table: "Customers".into(),
        field: "name".into(),
        properties: FieldProperties {
            caption: Some("Changed".into()),
            ..table().fields[1].properties.clone()
        },
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
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
    let edited = properties(&core);
    assert_eq!(edited[0].caption.as_deref(), Some("Identifier"));
    assert_eq!(edited[1].caption.as_deref(), Some("Changed"));
    assert_eq!(edited[2].calculated.as_deref(), Some("[id] * 2"));
    assert_eq!(edited[3].caption.as_deref(), Some("Note"));

    assert!(
        !core
            .dispatch(Command::SaveDesignDocument {
                path: sidecar.clone(),
            })
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    );
    core.dispatch(Command::CloseDatabase);

    let reopened = open_core(&database);
    assert_eq!(properties(&reopened), edited);
    let _ = &reopened;

    // Access-readable proxy: the plain Jet port still reads the file and
    // its rows (the property overlay lives in the sidecar, not LvProp).
    let mut port = Jet4Port::new();
    port.open(&database, None)
        .expect("the Jet file stays readable");
    assert!(port.tables().iter().any(|table| table.name == "Customers"));
    let rows = port
        .read_rows(
            "Customers",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("rows read");
    assert!(rows.is_empty(), "the table carries no rows");
    let _ = std::fs::remove_dir_all(&dir);
}
