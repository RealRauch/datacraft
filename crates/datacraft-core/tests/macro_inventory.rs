//! Macro inventory contract (step 17.1, item L4).
//!
//! The inventory is part of the serializable snapshot (agents read it via
//! `inspect`) and macros are addressable objects that open a read-only tab —
//! no execution semantics anywhere.

use std::path::PathBuf;

use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::macro_def::{MacroAction, MacroDef};
use datacraft_core::port::MockPort;
use datacraft_core::{ObjectId, ObjectKind, command::Command};

fn macro_port() -> MockPort {
    let mut port = MockPort::new();
    port.with_macros(vec![
        MacroDef {
            name: "AutoExec".to_owned(),
            actions: vec![
                MacroAction {
                    argument: "Northwind Traders".to_owned(),
                },
                MacroAction {
                    argument: "Startup Screen".to_owned(),
                },
            ],
            definition_decoded: true,
        },
        MacroDef {
            name: "Delete All Data".to_owned(),
            actions: vec![MacroAction {
                argument: "Delete * from [Invoices]".to_owned(),
            }],
            definition_decoded: true,
        },
    ]);
    port
}

fn opened_core() -> Core {
    let mut core = Core::new(Box::new(macro_port()));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("programmability.accdb"),
        password: None,
        source: None,
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    core
}

#[test]
fn the_snapshot_exposes_the_macro_inventory_with_nav_pane_test_ids() {
    let core = opened_core();
    let snapshot = core.inspect();
    let database = snapshot.database.expect("database is open");
    assert_eq!(database.macros.len(), 2);
    assert_eq!(database.macros[0].name, "AutoExec");
    assert_eq!(database.macros[0].actions.len(), 2);
    assert_eq!(database.macros[0].actions[1].argument, "Startup Screen");
    assert!(database.macros[0].definition_decoded);
    assert_eq!(database.macros[1].name, "Delete All Data");
    assert_eq!(
        database.macros[1].ui_test_id,
        "navpane.expanded.node.macro.delete all data"
    );
}

#[test]
fn macros_open_read_only_tabs_and_unknown_macros_fail_precisely() {
    let mut core = opened_core();
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Macro,
        "AutoExec",
    )));
    let [
        Event::TabOpened { object, .. },
        Event::ActiveTabChanged { .. },
    ] = events.as_slice()
    else {
        panic!("expected tab open pair, got {events:?}");
    };
    assert_eq!(object.kind, ObjectKind::Macro);
    assert_eq!(
        object.ui_test_id(),
        "node.macro.autoexec",
        "macro test ids derive from the kind slug"
    );
    assert_eq!(
        core.inspect().tabs[0].ui_test_id,
        "workspace.tab.macro.autoexec"
    );

    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Macro,
        "Missing",
    )));
    assert!(
        matches!(events.as_slice(), [Event::Failed { reason, .. }] if reason.contains("Missing")),
        "{events:?}"
    );
}

#[test]
fn the_macro_kind_slug_and_serde_name_are_stable() {
    assert_eq!(ObjectKind::Macro.slug(), "macro");
    let json = serde_json::to_string(&ObjectKind::Macro).expect("serializes");
    assert_eq!(json, "\"macro\"");
}
