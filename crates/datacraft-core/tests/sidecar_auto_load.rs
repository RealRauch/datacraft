//! Sidecar auto-load contract (phase 29, step 29.4, item H44).
//!
//! The `.datacraft` design sidecar next to a database (ADR-1) belongs to
//! that database: opening the database loads it automatically. A missing
//! sidecar is not special; a broken or foreign one surfaces as
//! `sidecar_error` in the snapshot without blocking the open (the same
//! observable-degradation pattern as `native_queries_error`, M10). RED
//! state: the auto-load and the snapshot field do not exist yet.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::QueryDef;
use datacraft_core::schema::{SCHEMA_VERSION, SchemaDocument};

/// Writes a sidecar next to the mock database path and returns the path.
fn with_sidecar(name: &str, sidecar: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "datacraft-core-sidecar-{}-{name}",
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
    let document = SchemaDocument {
        schema_version: SCHEMA_VERSION,
        database_name: database_name.to_owned(),
        tables: Vec::new(),
        queries: vec![QueryDef::new("Fast bikes", "Bikes")],
        forms: vec![datacraft_core::form::FormDef::new("Bike Finder")],
        reports: Vec::new(),
        views: Vec::new(),
    };
    document.to_json()
}

#[test]
fn a_sibling_sidecar_loads_with_the_open() {
    let db = with_sidecar("loads", &demo_sidecar("shop"));
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: db.clone(),
        password: None,
        source: None,
    });
    assert!(
        matches!(events[0], Event::DatabaseOpened { .. }),
        "{events:?}"
    );
    let loaded = match &events[1] {
        Event::DesignDocumentLoaded {
            forms,
            queries,
            reports,
            ..
        } => (*forms, *queries, *reports),
        other => panic!("the sidecar loaded: {other:?}"),
    };
    assert_eq!(loaded, (1, 1, 0), "the demo design layer is in");

    let db_state = core.inspect().database.expect("open");
    assert_eq!(db_state.forms.len(), 1);
    assert_eq!(db_state.queries.len(), 1);
    assert!(
        db_state.sidecar_error.is_none(),
        "{:?}",
        db_state.sidecar_error
    );
    let _ = std::fs::remove_dir_all(db.parent().expect("dir"));
}

#[test]
fn a_missing_sidecar_is_not_special() {
    let dir = std::env::temp_dir().join(format!(
        "datacraft-core-sidecar-{}-missing",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = dir.join("plain.mock");
    std::fs::write(&db, b"mock").expect("mock db file");

    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: db,
        password: None,
        source: None,
    });
    assert_eq!(events.len(), 1, "no DesignDocumentLoaded without a sidecar");
    assert!(
        core.inspect()
            .database
            .expect("open")
            .sidecar_error
            .is_none(),
        "a missing sidecar is not an error"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_sidecar_surfaces_its_error_and_keeps_the_open() {
    let db = with_sidecar("broken", "{ not json");
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: db.clone(),
        password: None,
        source: None,
    });
    assert!(
        matches!(events[0], Event::DatabaseOpened { .. }),
        "{events:?}"
    );
    assert_eq!(events.len(), 1, "no loaded event for a broken sidecar");
    let error = core
        .inspect()
        .database
        .expect("the open still succeeds")
        .sidecar_error
        .expect("the parse failure surfaces");
    assert!(!error.is_empty(), "{error}");
    let _ = std::fs::remove_dir_all(db.parent().expect("dir"));
}

#[test]
fn a_foreign_sidecar_is_reported_not_loaded() {
    let db = with_sidecar("foreign", &demo_sidecar("another_database"));
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: db.clone(),
        password: None,
        source: None,
    });
    let error = core
        .inspect()
        .database
        .expect("open")
        .sidecar_error
        .expect("the name mismatch surfaces");
    assert!(error.contains("another_database"), "{error}");
    let _ = std::fs::remove_dir_all(db.parent().expect("dir"));
}
