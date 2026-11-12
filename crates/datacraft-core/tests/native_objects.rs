//! Native Access form/report inventory in the core state (phase 74, step
//! 74.1; item M87).
//!
//! The port reports the catalog's form/report object types; the snapshot
//! carries them read-only for the navigation pane and agents. No
//! definition bytes are decoded and no tab opens.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::{MockPort, NativeObject, NativeObjectKind};

fn opened() -> Core {
    let mut port = MockPort::new();
    port.with_native_objects(vec![
        NativeObject {
            name: "Items Browser".into(),
            kind: NativeObjectKind::Form,
        },
        NativeObject {
            name: "Stock Summary".into(),
            kind: NativeObjectKind::Report,
        },
    ]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.accdb"),
        password: None,
        source: None,
    });
    core
}

#[test]
fn the_snapshot_lists_native_objects_read_only() {
    let core = opened();
    let database = core.inspect().database.expect("open");
    assert_eq!(database.native_objects.len(), 2);
    assert_eq!(database.native_objects[0].name, "Items Browser");
    assert_eq!(
        database.native_objects[0].kind,
        NativeObjectKind::Form,
        "the kind travels as data"
    );
    assert!(
        database.native_objects[0]
            .ui_test_id
            .contains("items browser")
    );
    assert_eq!(database.native_objects[1].kind, NativeObjectKind::Report);
    // They are not tables and open no tab.
    assert!(database.tables.is_empty());
}

#[test]
fn drivers_without_native_objects_report_none() {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.sqlite"),
        password: None,
        source: None,
    });
    assert!(
        core.inspect()
            .database
            .expect("open")
            .native_objects
            .is_empty()
    );
}
