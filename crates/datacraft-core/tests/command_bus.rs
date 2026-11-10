//! Contract suite for the command dispatch core (step 4.1, item H13).
//!
//! Locks ARCHITECTURE §5: `Core::dispatch` is a total reducer (failures
//! become `Event::Failed`), storage I/O happens behind the core-owned
//! `StoragePort`, and event order is deterministic. Uses only the shipped
//! `MockPort` — no storage crate involvement.

use std::path::PathBuf;

use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::port::{DatabaseFormat, MockPort, PortError};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_core::{ObjectId, command::Command};

// Split module (L30): tab contracts live beside this suite.
#[path = "command_bus/tabs.rs"]
mod tabs;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn core() -> Core {
    Core::new(Box::new(MockPort::new()))
}

fn opened_core() -> Core {
    let mut core = core();
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

fn customers_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn open_customers(core: &mut Core) -> TabId {
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
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
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// nav pane
// ---------------------------------------------------------------------------

#[test]
fn toggle_nav_pane_flips_visibility() {
    let mut core = core();
    assert_eq!(
        core.dispatch(Command::ToggleNavPane),
        vec![Event::NavPaneToggled { visible: false }]
    );
    assert_eq!(
        core.dispatch(Command::ToggleNavPane),
        vec![Event::NavPaneToggled { visible: true }]
    );
}

// ---------------------------------------------------------------------------
// open / close database
// ---------------------------------------------------------------------------

#[test]
fn open_database_reports_name_and_format() {
    let mut core = core();
    assert_eq!(
        core.dispatch(Command::OpenDatabase {
            path: PathBuf::from("shop.mdb"),
            password: None,
            source: None,
        }),
        vec![Event::DatabaseOpened {
            name: "shop".into(),
            format: DatabaseFormat::Jet4,
        }]
    );
}

#[test]
fn open_database_surfaces_port_failures_as_failed() {
    let mut core = Core::new(Box::new(MockPort::failing_open("disk unreadable")));
    let reason = failed_reason(&core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("broken.mdb"),
        password: None,
        source: None,
    }));
    assert!(reason.contains("disk unreadable"), "{reason}");
    // The core stays closed: a later open can succeed.
    core = Core::new(Box::new(MockPort::new()));
    assert!(matches!(
        core.dispatch(Command::OpenDatabase {
            path: PathBuf::from("ok.mdb"),
            password: None,
            source: None,
        })
        .as_slice(),
        [Event::DatabaseOpened { .. }]
    ));
}

#[test]
fn a_second_open_is_rejected_while_one_is_open() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("other.mdb"),
        password: None,
        source: None,
    }));
    assert!(reason.contains("already open"), "{reason}");
}

#[test]
fn close_database_closes_open_tabs_first() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    open_customers(&mut core);
    assert_eq!(
        core.dispatch(Command::CloseDatabase),
        vec![
            Event::TabClosed {
                tab_id: TabId::new(1)
            },
            Event::DatabaseClosed,
        ]
    );
    // Core stays usable afterwards.
    assert_eq!(
        core.dispatch(Command::ToggleNavPane),
        vec![Event::NavPaneToggled { visible: false }]
    );
}

#[test]
fn close_database_without_database_is_a_failed_noop() {
    let reason = failed_reason(&core().dispatch(Command::CloseDatabase));
    assert!(reason.contains("no database"), "{reason}");
}

// ---------------------------------------------------------------------------
// create / drop table
// ---------------------------------------------------------------------------

#[test]
fn create_table_emits_table_created() {
    let mut core = opened_core();
    assert_eq!(
        core.dispatch(Command::CreateTable {
            definition: customers_def(),
        }),
        vec![Event::TableCreated {
            table: "Customers".into()
        }]
    );
}

#[test]
fn create_table_requires_an_open_database() {
    let reason = failed_reason(&core().dispatch(Command::CreateTable {
        definition: customers_def(),
    }));
    assert!(reason.contains("no database"), "{reason}");
}

#[test]
fn create_table_rejects_invalid_definitions() {
    let mut core = opened_core();
    let invalid = TableDef::new("", vec![FieldDef::new("id", FieldKind::Currency)]);
    let reason = failed_reason(&core.dispatch(Command::CreateTable {
        definition: invalid,
    }));
    assert!(reason.contains("name"), "{reason}");
}

#[test]
fn create_table_rejects_duplicates_case_insensitively() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    let lower = TableDef::new("customers", vec![FieldDef::new("id", FieldKind::Currency)]);
    let reason = failed_reason(&core.dispatch(Command::CreateTable { definition: lower }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn drop_table_closes_dependent_tabs_before_dropping() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    open_customers(&mut core);
    assert_eq!(
        core.dispatch(Command::DropTable {
            table: "Customers".into(),
        }),
        vec![
            Event::TabClosed {
                tab_id: TabId::new(1)
            },
            Event::TableDropped {
                table: "Customers".into(),
            },
        ]
    );
}

#[test]
fn drop_table_is_case_insensitive() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    assert_eq!(
        core.dispatch(Command::DropTable {
            table: "customers".into(),
        }),
        vec![Event::TableDropped {
            table: "Customers".into(),
        }]
    );
}

#[test]
fn drop_unknown_table_fails() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::DropTable {
        table: "Ghost".into(),
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn drop_table_requires_an_open_database() {
    let reason = failed_reason(&core().dispatch(Command::DropTable {
        table: "Ghost".into(),
    }));
    assert!(reason.contains("no database"), "{reason}");
}

// ---------------------------------------------------------------------------
// tabs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// port error taxonomy
// ---------------------------------------------------------------------------

#[test]
fn port_write_failures_surface_as_failed_events() {
    let mut core = Core::new(Box::new(MockPort::failing_writes("read-only file")));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("ro.mdb"),
        password: None,
        source: None,
    });
    let reason = failed_reason(&core.dispatch(Command::CreateTable {
        definition: customers_def(),
    }));
    assert!(reason.contains("read-only file"), "{reason}");
}

#[test]
fn port_error_displays_are_precise() {
    let open = PortError::OpenFailed {
        detail: "bad magic".into(),
    };
    assert!(open.to_string().contains("bad magic"));
    let missing = PortError::TableNotFound("X".into());
    assert!(missing.to_string().contains('X'));
    let write = PortError::WriteFailed {
        detail: "locked".into(),
    };
    assert!(write.to_string().contains("locked"));
    let unsupported = PortError::Unsupported {
        reason: "encrypted".into(),
    };
    assert!(unsupported.to_string().contains("encrypted"));
}

#[test]
fn the_relationship_mapper_toggles_observably() {
    let mut core = core();
    assert_eq!(
        core.dispatch(Command::ToggleRelationshipMapper),
        vec![Event::RelationshipMapperToggled { open: true }]
    );
    assert_eq!(
        core.dispatch(Command::ToggleRelationshipMapper),
        vec![Event::RelationshipMapperToggled { open: false }]
    );
}

#[test]
fn closing_the_database_closes_the_relationship_mapper() {
    let mut core = opened_core();
    core.dispatch(Command::ToggleRelationshipMapper);
    core.dispatch(Command::CloseDatabase);
    assert!(!core.state().relationship_mapper_open);
}
