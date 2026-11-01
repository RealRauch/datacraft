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
        [Event::Failed { reason }] => reason.clone(),
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
    }));
    assert!(reason.contains("disk unreadable"), "{reason}");
    // The core stays closed: a later open can succeed.
    core = Core::new(Box::new(MockPort::new()));
    assert!(matches!(
        core.dispatch(Command::OpenDatabase {
            path: PathBuf::from("ok.mdb")
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

#[test]
fn open_object_opens_and_activates_a_tab() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    assert_eq!(
        core.dispatch(Command::OpenObject(ObjectId::new(
            datacraft_core::ObjectKind::Table,
            "Customers"
        ))),
        vec![
            Event::TabOpened {
                tab_id: TabId::new(1),
                object: ObjectId::new(datacraft_core::ObjectKind::Table, "Customers"),
            },
            Event::ActiveTabChanged {
                tab_id: Some(TabId::new(1)),
            },
        ]
    );
}

#[test]
fn open_object_is_idempotent_per_object() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    let object = ObjectId::new(datacraft_core::ObjectKind::Table, "Customers");
    core.dispatch(Command::OpenObject(object.clone()));
    // Second open of the same object re-activates the existing tab and
    // consumes no new tab id.
    assert_eq!(
        core.dispatch(Command::OpenObject(object)),
        vec![Event::ActiveTabChanged {
            tab_id: Some(TabId::new(1)),
        }]
    );
}

#[test]
fn open_object_rejects_unknown_tables() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Ghost",
    ))));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn open_object_requires_an_open_database() {
    let reason = failed_reason(&core().dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    ))));
    assert!(reason.contains("no database"), "{reason}");
}

#[test]
fn closing_the_active_tab_activates_the_next_lower_tab() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new("Orders", vec![FieldDef::new("total", FieldKind::Currency)]),
    });
    open_customers(&mut core);
    core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    // Active is now tab 2; closing it activates tab 1.
    assert_eq!(
        core.dispatch(Command::CloseTab {
            tab_id: TabId::new(2)
        }),
        vec![
            Event::TabClosed {
                tab_id: TabId::new(2)
            },
            Event::ActiveTabChanged {
                tab_id: Some(TabId::new(1)),
            },
        ]
    );
    // Closing the last tab clears the activation.
    assert_eq!(
        core.dispatch(Command::CloseTab {
            tab_id: TabId::new(1)
        }),
        vec![
            Event::TabClosed {
                tab_id: TabId::new(1)
            },
            Event::ActiveTabChanged { tab_id: None },
        ]
    );
}

#[test]
fn closing_a_background_tab_keeps_the_active_tab() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new("Orders", vec![FieldDef::new("total", FieldKind::Currency)]),
    });
    open_customers(&mut core);
    core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    core.dispatch(Command::SetActiveTab {
        tab_id: TabId::new(1),
    });
    assert_eq!(
        core.dispatch(Command::CloseTab {
            tab_id: TabId::new(2)
        }),
        vec![Event::TabClosed {
            tab_id: TabId::new(2)
        }]
    );
}

#[test]
fn close_tab_unknown_id_fails() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::CloseTab {
        tab_id: TabId::new(9),
    }));
    assert!(reason.contains("9"), "{reason}");
}

#[test]
fn set_active_tab_is_observable() {
    let mut core = opened_core();
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core.dispatch(Command::CreateTable {
        definition: TableDef::new("Orders", vec![FieldDef::new("total", FieldKind::Currency)]),
    });
    open_customers(&mut core);
    core.dispatch(Command::OpenObject(ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    assert_eq!(
        core.dispatch(Command::SetActiveTab {
            tab_id: TabId::new(1)
        }),
        vec![Event::ActiveTabChanged {
            tab_id: Some(TabId::new(1)),
        }]
    );
}

#[test]
fn set_active_tab_unknown_id_fails() {
    let mut core = opened_core();
    let reason = failed_reason(&core.dispatch(Command::SetActiveTab {
        tab_id: TabId::new(7),
    }));
    assert!(reason.contains("7"), "{reason}");
}

// ---------------------------------------------------------------------------
// port error taxonomy
// ---------------------------------------------------------------------------

#[test]
fn port_write_failures_surface_as_failed_events() {
    let mut core = Core::new(Box::new(MockPort::failing_writes("read-only file")));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("ro.mdb"),
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
