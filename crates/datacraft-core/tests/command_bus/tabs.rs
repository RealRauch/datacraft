//! Tab-opening and activation contracts (split from `command_bus.rs`
//! to stay under the file-size gate; L30).

use super::*;

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
