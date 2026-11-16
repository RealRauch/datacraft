//! Contract suite for the idle-tab reuse policy (step 88.3, item L49).
//!
//! Activating an object that is already open re-activates its tab —
//! never a second tab — and an explicit mode is applied to the reused
//! tab. The behavior shipped with the tab reducers; these pins close the
//! untested contract (the reuse path had no RED to evidence — the pins
//! are the deliverable, documented per the M65 precedent).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};

fn table_def(name: &str) -> TableDef {
    TableDef::new(
        name,
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
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
        source: None,
    });
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    core
}

fn open(core: &mut Core, name: &str) -> datacraft_core::ids::TabId {
    match core
        .dispatch(Command::OpenObject(ObjectId::new(ObjectKind::Table, name)))
        .as_slice()
    {
        [Event::TabOpened { tab_id, .. }, ..] => *tab_id,
        other => panic!("expected tab open events, got {other:?}"),
    }
}

#[test]
fn activating_an_open_object_reactivates_its_tab() {
    let mut core = opened_core();
    let _first = open(&mut core, "Customers");
    let second = open(&mut core, "Orders");
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Items",
    )));
    assert_eq!(core.inspect().tabs.len(), 3, "three documents are open");

    // Re-activation: one ActiveTabChanged, no TabOpened, no second tab.
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Orders",
    )));
    assert_eq!(
        events,
        vec![Event::ActiveTabChanged {
            tab_id: Some(second)
        }],
        "an idle object must activate its existing tab, not open another one"
    );
    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs.len(), 3, "no tab was added");
    assert_eq!(snapshot.active_tab, Some(second));
}

#[test]
fn activating_an_open_object_in_a_requested_mode_reuses_the_tab() {
    let mut core = opened_core();
    let _first = open(&mut core, "Customers");
    let second = open(&mut core, "Orders");
    let events = core.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Table, "Orders"),
        mode: TabMode::Design,
    });
    assert_eq!(
        events,
        vec![
            Event::ActiveTabChanged {
                tab_id: Some(second)
            },
            Event::TabModeSet {
                tab_id: second,
                mode: TabMode::Design
            },
        ],
        "the requested mode applies to the reused tab"
    );
    let snapshot = core.inspect();
    assert_eq!(snapshot.tabs.len(), 2, "no tab was added");
    assert_eq!(snapshot.tabs[1].mode, TabMode::Design);
}

#[test]
fn name_case_of_an_open_object_resolves_to_its_tab() {
    let mut core = opened_core();
    let _first = open(&mut core, "Customers");
    let second = open(&mut core, "Orders");
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "ORDERS",
    )));
    assert_eq!(
        events,
        vec![Event::ActiveTabChanged {
            tab_id: Some(second)
        }],
        "the object match is case-insensitive like the open path"
    );
    assert_eq!(core.inspect().tabs.len(), 2);
}
