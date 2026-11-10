//! Snapshot source and sidecar contract (step 65.2, item L8): the open
//! session remembers its `SourceSpec` for every client, the snapshot
//! carries it wire-additively, and the file-bound sidecar (ADR-1) applies
//! to filesystem-backed sessions — a `file://` URI session is the same
//! file and auto-loads its sibling sidecar like a plain path open.

use std::path::PathBuf;

use datacraft_core::command::{Command, SourceSpec};
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::QueryDef;
use datacraft_core::schema::{SCHEMA_VERSION, SchemaDocument};
use datacraft_core::snapshot::UiStateSnapshot;

/// Writes a mock database with a sibling sidecar and returns its path.
fn with_sidecar(name: &str, sidecar: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "datacraft-core-source-sidecar-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = dir.join("shop.mock");
    std::fs::write(&db, b"mock").expect("mock db file");
    std::fs::write(dir.join("shop.datacraft"), sidecar).expect("sidecar");
    db
}

fn demo_sidecar(database_name: &str) -> String {
    SchemaDocument {
        schema_version: SCHEMA_VERSION,
        database_name: database_name.to_owned(),
        tables: Vec::new(),
        queries: vec![QueryDef::new("Fast bikes", "Bikes")],
        forms: vec![datacraft_core::form::FormDef::new("Bike Finder")],
        reports: Vec::new(),
    }
    .to_json()
}

fn opened(core: &Core) -> &datacraft_core::snapshot::DatabaseSnapshot {
    core.inspect()
        .database
        .as_ref()
        .expect("database must be open")
}

#[test]
fn the_snapshot_carries_the_opening_source_spec() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let uri = if cfg!(windows) {
        "file:///C:/dir/shop.mock"
    } else {
        "file:///dir/shop.mock"
    };
    let plain_path = if cfg!(windows) {
        PathBuf::from("C:/dir/shop.mock")
    } else {
        PathBuf::from("/dir/shop.mock")
    };
    let events = core.dispatch(Command::OpenDatabase {
        path: plain_path,
        password: None,
        source: Some(SourceSpec::Uri(uri.to_owned())),
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    assert_eq!(
        opened(&core).source,
        Some(SourceSpec::Uri(uri.to_owned())),
        "the snapshot must report the spec the session was opened with"
    );

    // A plain path open reports no source — the historical shape.
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mock"),
        password: None,
        source: None,
    });
    assert_eq!(opened(&core).source, None);
}

#[test]
fn a_file_uri_session_auto_loads_the_sibling_sidecar() {
    // The sidecar is file-bound (ADR-1): a `file://` session is the same
    // file, so its sibling sidecar loads exactly like for a plain path.
    let db = with_sidecar("uri-loads", &demo_sidecar("shop"));
    let path = db.display().to_string().replace('\\', "/");
    let uri = format!("file:///{path}");
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: db,
        password: None,
        source: Some(SourceSpec::Uri(uri)),
    });
    assert!(
        matches!(
            events.as_slice(),
            [
                Event::DatabaseOpened { .. },
                Event::DesignDocumentLoaded { .. }
            ]
        ),
        "the uri open must auto-load the sibling sidecar: {events:?}"
    );
    assert!(opened(&core).queries.iter().any(|q| q.name == "Fast bikes"));
}

#[test]
fn the_snapshot_source_round_trips_through_the_wire() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mock"),
        password: None,
        source: Some(SourceSpec::Uri("file:///dir/shop.mock".into())),
    });
    let snap = core.inspect();
    let raw = serde_json::to_string(&snap).expect("serialize");
    let back: UiStateSnapshot = serde_json::from_str(&raw).expect("deserialize");
    assert_eq!(back, snap);
    assert_eq!(
        back.database.expect("open").source,
        Some(SourceSpec::Uri("file:///dir/shop.mock".into()))
    );
}

#[test]
fn the_historical_snapshot_shape_without_source_still_deserializes() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mock"),
        password: None,
        source: None,
    });
    let snap = core.inspect();
    let mut value = serde_json::to_value(&snap).expect("serialize");
    value
        .get_mut("database")
        .and_then(|db| db.as_object_mut())
        .expect("database object")
        .remove("source");
    let back: UiStateSnapshot =
        serde_json::from_value(value).expect("historical snapshot JSON keeps deserializing");
    assert_eq!(back.database.expect("open").source, None);
}
