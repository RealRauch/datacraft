//! Relationship creation/deletion over the bus (step 32.3, item M16).
//!
//! The commands are capability-gated (`relationships: Yes`), validate the
//! referenced tables/columns, refuse duplicates and refresh the snapshot's
//! relationship list. Drivers answer the write; the mock records it.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DriverCapabilities, MockPort, Relationship, Support};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn relationship() -> Relationship {
    Relationship {
        table: "Orders".into(),
        column: "customer_id".into(),
        related_table: "Customers".into(),
        related_column: "id".into(),
    }
}

fn core_with(caps: DriverCapabilities) -> Core {
    let customers = TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let orders = TableDef::new(
        "Orders",
        vec![FieldDef::new(
            "customer_id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut port = MockPort::with_tables(vec![customers, orders]);
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    core
}

fn capable() -> Core {
    core_with(DriverCapabilities {
        relationships: Support::Yes,
        ..DriverCapabilities::full()
    })
}

fn failure(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected Failed: {events:?}"))
}

#[test]
fn create_and_delete_round_trip_through_the_snapshot() {
    let mut core = capable();
    let events = core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    assert!(
        matches!(events.first(), Some(Event::RelationshipCreated { .. })),
        "{events:?}"
    );
    assert_eq!(
        core.inspect().database.expect("open").relationships,
        vec![relationship()]
    );

    let events = core.dispatch(Command::DeleteRelationship {
        relationship: relationship(),
    });
    assert!(
        matches!(events.first(), Some(Event::RelationshipDeleted { .. })),
        "{events:?}"
    );
    assert!(
        core.inspect()
            .database
            .expect("open")
            .relationships
            .is_empty()
    );
}

#[test]
fn duplicates_and_missing_targets_are_refused() {
    let mut core = capable();
    core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    });
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    }));
    assert!(reason.contains("duplicate"), "{reason}");

    let mut missing_table = relationship();
    missing_table.table = "Ghost".into();
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: missing_table,
    }));
    assert!(reason.contains("Ghost"), "{reason}");

    let mut missing_column = relationship();
    missing_column.column = "ghost".into();
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: missing_column,
    }));
    assert!(reason.contains("ghost"), "{reason}");

    let self_reference = Relationship {
        table: "Customers".into(),
        column: "id".into(),
        related_table: "Customers".into(),
        related_column: "id".into(),
    };
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: self_reference,
    }));
    assert!(reason.contains("cannot reference itself"), "{reason}");

    let reason = failure(&core.dispatch(Command::DeleteRelationship {
        relationship: Relationship {
            table: "Orders".into(),
            column: "customer_id".into(),
            related_table: "Ghost".into(),
            related_column: "id".into(),
        },
    }));
    assert!(reason.contains("not found"), "{reason}");
}

#[test]
fn the_capability_gate_precedes_the_port() {
    for support in [Support::ReadOnly, Support::No] {
        let mut port = MockPort::with_tables(vec![]);
        port.with_capabilities(DriverCapabilities {
            relationships: support,
            ..DriverCapabilities::full()
        });
        let mut core = Core::new(Box::new(port));
        core.dispatch(Command::OpenDatabase {
            path: "shop.mdb".into(),
            password: None,
        });
        let reason = failure(&core.dispatch(Command::CreateRelationship {
            relationship: relationship(),
        }));
        assert!(
            reason.contains("cannot create relationships"),
            "{support:?}: {reason}"
        );
        let reason = failure(&core.dispatch(Command::DeleteRelationship {
            relationship: relationship(),
        }));
        assert!(
            reason.contains("cannot delete relationships"),
            "{support:?}: {reason}"
        );
    }
}

#[test]
fn a_port_failure_is_reported_and_changes_nothing() {
    let customers = TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let orders = TableDef::new(
        "Orders",
        vec![FieldDef::new(
            "customer_id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut port = MockPort::with_tables(vec![customers, orders]);
    port.with_write_failure("disk full");
    port.with_capabilities(DriverCapabilities {
        relationships: Support::Yes,
        ..DriverCapabilities::full()
    });
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    let reason = failure(&core.dispatch(Command::CreateRelationship {
        relationship: relationship(),
    }));
    assert!(reason.contains("disk full"), "{reason}");
}
