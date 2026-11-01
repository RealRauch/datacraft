//! VBA project contract (step 17.2, item L5): snapshot inventory, module
//! tabs and the bus command that returns one module's source.
//!
//! No execution semantics: the source is extracted, stored and displayed
//! only (ADR-2).

use std::path::PathBuf;

use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};
use datacraft_core::{ObjectId, ObjectKind, command::Command};

fn project() -> VbaProject {
    VbaProject {
        modules: vec![
            VbaModule {
                name: "Utilities".to_owned(),
                kind: VbaModuleKind::Standard,
                source: "Attribute VB_Name = \"Utilities\"\nOption Compare Database\n".to_owned(),
            },
            VbaModule {
                name: "Form_Sales".to_owned(),
                kind: VbaModuleKind::Form,
                source: "Attribute VB_Name = \"Form_Sales\"\n".to_owned(),
            },
        ],
    }
}

fn opened_core() -> Core {
    let mut port = MockPort::new();
    port.with_vba(project());
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("northwind.accdb"),
        password: None,
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    core
}

#[test]
fn the_snapshot_lists_vba_modules_with_kind_and_line_count() {
    let core = opened_core();
    let snapshot = core.inspect();
    let database = snapshot.database.expect("database is open");
    let vba = database.vba.expect("VBA project in the snapshot");
    assert_eq!(vba.modules.len(), 2);
    assert_eq!(vba.modules[0].name, "Utilities");
    assert_eq!(vba.modules[0].kind, VbaModuleKind::Standard);
    assert_eq!(vba.modules[0].lines, 2);
    assert_eq!(
        vba.modules[0].ui_test_id,
        "navpane.expanded.node.module.utilities"
    );
    assert_eq!(vba.modules[1].kind, VbaModuleKind::Form);
    assert_eq!(database.vba_error, None);
}

#[test]
fn modules_open_read_only_tabs_and_load_their_source_over_the_bus() {
    let mut core = opened_core();
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Module,
        "Utilities",
    )));
    let [
        Event::TabOpened { object, .. },
        Event::ActiveTabChanged { .. },
    ] = events.as_slice()
    else {
        panic!("expected tab open pair, got {events:?}");
    };
    assert_eq!(object.kind, ObjectKind::Module);
    assert_eq!(
        core.inspect().tabs[0].ui_test_id,
        "workspace.tab.module.utilities"
    );

    let events = core.dispatch(Command::LoadVbaModule {
        module: "Utilities".to_owned(),
    });
    match events.as_slice() {
        [Event::VbaModuleLoaded { module }] => {
            assert_eq!(module.name, "Utilities");
            assert!(module.source.contains("Option Compare Database"));
        }
        other => panic!("expected module load, got {other:?}"),
    }
    assert_eq!(
        core.vba_module("utilities")
            .expect("case-insensitive lookup")
            .source
            .lines()
            .count(),
        2
    );

    let events = core.dispatch(Command::LoadVbaModule {
        module: "Missing".to_owned(),
    });
    assert!(
        matches!(events.as_slice(), [Event::Failed { reason }] if reason.contains("Missing")),
        "{events:?}"
    );
}

#[test]
fn a_failed_extraction_stays_visible_and_does_not_block_opening() {
    let mut port = MockPort::new();
    port.with_vba_error("VBA dir stream is corrupt");
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("broken.accdb"),
        password: None,
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    let snapshot = core.inspect();
    let database = snapshot.database.expect("database is open");
    assert!(database.vba.is_none());
    let vba_error = database.vba_error.as_deref().expect("error is visible");
    assert!(
        vba_error.contains("VBA dir stream is corrupt"),
        "{vba_error}"
    );
    let events = core.dispatch(Command::LoadVbaModule {
        module: "Utilities".to_owned(),
    });
    assert!(
        matches!(events.as_slice(), [Event::Failed { reason }] if reason.contains("dir stream")),
        "{events:?}"
    );
}

#[test]
fn the_module_kind_slug_and_serde_name_are_stable() {
    assert_eq!(ObjectKind::Module.slug(), "module");
    assert_eq!(
        serde_json::to_string(&ObjectKind::Module).expect("serializes"),
        "\"module\""
    );
    assert_eq!(
        serde_json::to_string(&VbaModuleKind::Standard).expect("serializes"),
        "\"standard\""
    );
}
