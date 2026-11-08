//! Contract suite for per-tab view modes (step 21.1, item U5).
//!
//! The view mode is document state, not a UI setting (ADR-4): it lives in
//! the core, changes only through `SetTabMode` and is projected per tab in
//! `TabSnapshot` — headless clients see and drive exactly what the GUI does.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::macro_def::MacroDef;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::state::TabMode;
use datacraft_core::transport::handle_json;
use datacraft_core::{ObjectId, ObjectKind};
use serde_json::{Value, json};

fn table_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn opened_core() -> Core {
    let mut port = MockPort::with_tables(vec![table_def()]);
    port.with_macros(vec![MacroDef {
        name: "AutoExec".to_owned(),
        actions: Vec::new(),
        definition_decoded: true,
    }]);
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    core
}

fn open(core: &mut Core, kind: ObjectKind, name: &str) -> TabId {
    let events = core.dispatch(Command::OpenObject(ObjectId::new(kind, name)));
    match events.as_slice() {
        [
            Event::TabOpened { tab_id, .. },
            Event::ActiveTabChanged { .. },
        ] => *tab_id,
        other => panic!("expected tab open pair, got {other:?}"),
    }
}

fn mode_of(core: &Core, tab_id: TabId) -> TabMode {
    core.inspect()
        .tabs
        .iter()
        .find(|tab| tab.id == tab_id)
        .map(|tab| tab.mode)
        .expect("tab in snapshot")
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn opening_an_object_tab_starts_in_datasheet_mode() {
    let mut core = opened_core();
    let tab_id = open(&mut core, ObjectKind::Table, "Customers");
    assert_eq!(mode_of(&core, tab_id), TabMode::Datasheet);
    let snapshot = core.inspect();
    let tab = snapshot.tabs.iter().find(|tab| tab.id == tab_id).unwrap();
    assert!(tab.has_design_view, "a table tab has a design view");
}

#[test]
fn set_tab_mode_switches_the_projected_mode() {
    let mut core = opened_core();
    let tab_id = open(&mut core, ObjectKind::Table, "Customers");
    let events = core.dispatch(Command::SetTabMode {
        tab_id,
        mode: TabMode::Design,
    });
    assert_eq!(
        events,
        vec![Event::TabModeSet {
            tab_id,
            mode: TabMode::Design
        }]
    );
    assert_eq!(mode_of(&core, tab_id), TabMode::Design);
    let events = core.dispatch(Command::SetTabMode {
        tab_id,
        mode: TabMode::Datasheet,
    });
    assert_eq!(
        events,
        vec![Event::TabModeSet {
            tab_id,
            mode: TabMode::Datasheet
        }]
    );
    assert_eq!(mode_of(&core, tab_id), TabMode::Datasheet);
}

#[test]
fn set_tab_mode_on_an_unknown_tab_fails_precisely() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::SetTabMode {
        tab_id: TabId::new(99),
        mode: TabMode::Design,
    }));
    assert!(reason.contains("unknown tab"), "{reason}");
}

#[test]
fn set_tab_mode_rejects_objects_without_a_design_view() {
    let mut core = opened_core();
    let tab_id = open(&mut core, ObjectKind::Macro, "AutoExec");
    let snapshot = core.inspect();
    let tab = snapshot.tabs.iter().find(|tab| tab.id == tab_id).unwrap();
    assert!(!tab.has_design_view, "a macro tab has no design view");
    let reason = failed_reason(&core.dispatch(Command::SetTabMode {
        tab_id,
        mode: TabMode::Design,
    }));
    assert!(reason.contains("design view"), "{reason}");
    assert_eq!(mode_of(&core, tab_id), TabMode::Datasheet);
}

#[test]
fn set_tab_mode_round_trips_over_the_json_rpc_wire() {
    let mut core = opened_core();
    let tab_id = open(&mut core, ObjectKind::Table, "Customers");
    let raw = handle_json(
        &mut core,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "dispatch",
            "params": {"SetTabMode": {"tab_id": {"value": tab_id.value()}, "mode": "Design"}},
        })
        .to_string(),
    );
    let response: Value = serde_json::from_str(&raw).expect("response parses");
    assert_eq!(
        response["result"]["events"][0]["TabModeSet"]["mode"],
        "Design"
    );
    assert_eq!(response["result"]["snapshot"]["tabs"][0]["mode"], "Design");
}

#[test]
fn open_object_in_mode_opens_the_tab_directly_in_that_view() {
    let mut core = opened_core();
    let events = core.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Table, "Customers"),
        mode: TabMode::Design,
    });
    match events.as_slice() {
        [
            Event::TabOpened { tab_id, .. },
            Event::ActiveTabChanged { tab_id: active },
        ] => {
            assert_eq!(active, &Some(*tab_id));
            assert_eq!(mode_of(&core, *tab_id), TabMode::Design);
        }
        other => panic!("expected tab open pair, got {other:?}"),
    }
}

#[test]
fn open_object_in_mode_switches_an_existing_tab_in_one_command() {
    let mut core = opened_core();
    let tab_id = open(&mut core, ObjectKind::Table, "Customers");
    let events = core.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Table, "Customers"),
        mode: TabMode::Design,
    });
    assert_eq!(
        events,
        vec![
            Event::ActiveTabChanged {
                tab_id: Some(tab_id)
            },
            Event::TabModeSet {
                tab_id,
                mode: TabMode::Design
            }
        ]
    );
    assert_eq!(mode_of(&core, tab_id), TabMode::Design);
}

#[test]
fn open_object_in_mode_rejects_kinds_without_a_design_view() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Macro, "AutoExec"),
        mode: TabMode::Design,
    }));
    assert!(reason.contains("design view"), "{reason}");
    assert!(
        core.inspect().tabs.is_empty(),
        "a rejected open must not create a tab"
    );
}

#[test]
fn open_object_in_mode_round_trips_over_the_json_rpc_wire() {
    let mut core = opened_core();
    let raw = handle_json(
        &mut core,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "dispatch",
            "params": {"OpenObjectInMode": {
                "object": {"kind": "table", "name": "Customers"},
                "mode": "Design"
            }},
        })
        .to_string(),
    );
    let response: Value = serde_json::from_str(&raw).expect("response parses");
    assert_eq!(response["result"]["snapshot"]["tabs"][0]["mode"], "Design");
}
