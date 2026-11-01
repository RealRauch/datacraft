//! Contract suite for bulk tab closing (step 21.6, item U10).
//!
//! Close Others / Close All are ordinary bus commands — the UI never loops
//! over tabs itself, so GUI, CLI and MCP share exactly the same semantics.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::transport::handle_json;
use datacraft_core::{ObjectId, ObjectKind};
use serde_json::{Value, json};

fn table_def(name: &str) -> TableDef {
    TableDef::new(
        name,
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn opened_core() -> Core {
    let port = MockPort::with_tables(vec![
        table_def("Customers"),
        table_def("Orders"),
        table_def("Items"),
    ]);
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

fn open(core: &mut Core, name: &str) -> TabId {
    let events = core.dispatch(Command::OpenObject(ObjectId::new(ObjectKind::Table, name)));
    match events.as_slice() {
        [
            Event::TabOpened { tab_id, .. },
            Event::ActiveTabChanged { .. },
        ] => *tab_id,
        other => panic!("expected tab open pair, got {other:?}"),
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn close_other_tabs_keeps_the_addressed_tab_and_activates_it() {
    let mut core = opened_core();
    let customers = open(&mut core, "Customers");
    let orders = open(&mut core, "Orders");
    let items = open(&mut core, "Items");
    let events = core.dispatch(Command::CloseOtherTabs { tab_id: customers });
    assert_eq!(
        events,
        vec![
            Event::TabClosed { tab_id: orders },
            Event::TabClosed { tab_id: items },
            Event::ActiveTabChanged {
                tab_id: Some(customers)
            },
        ]
    );
    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs.len(), 1);
    assert_eq!(snapshot.tabs[0].id, customers);
    assert_eq!(snapshot.active_tab, Some(customers));
}

#[test]
fn close_other_tabs_on_the_active_tab_stays_silent_about_activation() {
    let mut core = opened_core();
    let customers = open(&mut core, "Customers");
    let items = open(&mut core, "Items");
    let events = core.dispatch(Command::CloseOtherTabs { tab_id: items });
    assert_eq!(events, vec![Event::TabClosed { tab_id: customers }]);
    assert_eq!(core.inspect().active_tab, Some(items));
}

#[test]
fn close_other_tabs_rejects_an_unknown_tab() {
    let mut core = opened_core();
    open(&mut core, "Customers");
    let reason = failed_reason(&core.dispatch(Command::CloseOtherTabs {
        tab_id: TabId::new(99),
    }));
    assert!(reason.contains("unknown tab"), "{reason}");
    assert_eq!(core.inspect().tabs.len(), 1, "nothing was closed");
}

#[test]
fn close_all_tabs_empties_the_workspace_in_opening_order() {
    let mut core = opened_core();
    let customers = open(&mut core, "Customers");
    let orders = open(&mut core, "Orders");
    let items = open(&mut core, "Items");
    let events = core.dispatch(Command::CloseAllTabs);
    assert_eq!(
        events,
        vec![
            Event::TabClosed { tab_id: customers },
            Event::TabClosed { tab_id: orders },
            Event::TabClosed { tab_id: items },
            Event::ActiveTabChanged { tab_id: None },
        ]
    );
    let snapshot = core.inspect();
    assert!(snapshot.tabs.is_empty());
    assert_eq!(snapshot.active_tab, None);
}

#[test]
fn close_all_tabs_without_tabs_fails_precisely() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::CloseAllTabs));
    assert!(reason.contains("no tabs"), "{reason}");
}

#[test]
fn bulk_close_commands_round_trip_over_the_json_rpc_wire() {
    let mut core = opened_core();
    let customers = open(&mut core, "Customers");
    let _orders = open(&mut core, "Orders");
    let raw = handle_json(
        &mut core,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "dispatch",
            "params": {"CloseOtherTabs": {"tab_id": {"value": customers.value()}}},
        })
        .to_string(),
    );
    let response: Value = serde_json::from_str(&raw).expect("response parses");
    assert_eq!(
        response["result"]["snapshot"]["tabs"][0]["id"]["value"],
        customers.value()
    );
    let raw = handle_json(
        &mut core,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "dispatch", "params": {"CloseAllTabs": null}})
            .to_string(),
    );
    let response: Value = serde_json::from_str(&raw).expect("response parses");
    assert_eq!(
        response["result"]["snapshot"]["tabs"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
