//! CreateDatabase command contract (phase 29, step 29.1, item H44).
//!
//! Creating a database file is a bus command like every user-visible
//! action: exactly one command, one `DatabaseCreated` event, independent
//! of the open-database state. RED state: `Command::CreateDatabase` and
//! `Event::DatabaseCreated` do not exist yet — the suite fails to compile
//! until the command lands.

use datacraft_core::availability::DisabledReason;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{DatabaseFormat, DriverCapabilities, MockPort};

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-core-create-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn creating_a_jet4_database_reports_one_event_and_opens_nothing() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let path = scratch("jet4.mdb");
    let events = core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    assert_eq!(
        events,
        vec![Event::DatabaseCreated {
            path: path.clone(),
            format: DatabaseFormat::Jet4,
        }]
    );
    assert!(core.inspect().database.is_none(), "creation never opens");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sqlite_databases_are_created_through_the_same_command() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let path = scratch("notes.sqlite");
    let events = core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    assert_eq!(
        events,
        vec![Event::DatabaseCreated {
            path: path.clone(),
            format: DatabaseFormat::Sqlite,
        }]
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn creation_is_independent_of_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    assert!(
        matches!(events[0], Event::DatabaseOpened { .. }),
        "{events:?}"
    );

    let path = scratch("while-open.mdb");
    let events = core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    assert!(
        matches!(events[0], Event::DatabaseCreated { .. }),
        "creation must not require closing the open database: {events:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn port_failures_become_failed_events() {
    let mut core = Core::new(Box::new(MockPort::failing_writes("disk is full")));
    let events = core.dispatch(Command::CreateDatabase {
        path: scratch("full.mdb"),
        format: DatabaseFormat::Jet4,
    });
    match events.first() {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("disk is full"), "{reason}"),
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

#[test]
fn create_database_commands_round_trip_through_json() {
    let command = Command::CreateDatabase {
        path: "new.mdb".into(),
        format: DatabaseFormat::Sqlite,
    };
    let json = serde_json::to_string(&command).expect("serialize command");
    assert!(json.contains("\"format\":\"sqlite\""), "{json}");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(back, command);
}

/// The suffix/format agreement (step 75.3, item L43): a path naming the
/// other creatable family's extension answers a precise `InvalidInput`
/// before the port — the command still carries the format explicitly.
#[test]
fn a_suffix_from_the_other_family_is_refused_before_the_port() {
    let mut core = Core::new(Box::new(MockPort::failing_writes("the port was reached")));
    let path = scratch("mismatch.mdb");
    let events = core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Sqlite,
    });
    match events.first() {
        Some(Event::Failed { code, reason }) => {
            assert_eq!(*code, FailureCode::InvalidInput, "{reason}");
            assert!(
                reason.contains(".mdb") && reason.contains("SQLite"),
                "the reason names the suffix and the format: {reason}"
            );
            assert!(
                !reason.contains("the port was reached"),
                "the check precedes the port: {reason}"
            );
        }
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

#[test]
fn jet4_creation_refuses_a_sqlite_suffix() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::CreateDatabase {
        path: scratch("mismatch.sqlite"),
        format: DatabaseFormat::Jet4,
    });
    match events.first() {
        Some(Event::Failed { code, reason }) => {
            assert_eq!(*code, FailureCode::InvalidInput, "{reason}");
            assert!(
                reason.contains(".sqlite") && reason.contains("Jet4"),
                "{reason}"
            );
        }
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

/// Matching extensions are accepted case-insensitively; unknown or missing
/// extensions stay legal because the command carries the format.
#[test]
fn matching_and_unknown_suffixes_are_accepted() {
    for (name, format) in [
        ("ok.mdb", DatabaseFormat::Jet4),
        ("ok.SQLITE", DatabaseFormat::Sqlite),
        ("notes.db", DatabaseFormat::Sqlite),
        ("notes", DatabaseFormat::Jet4),
    ] {
        let mut core = Core::new(Box::new(MockPort::new()));
        let events = core.dispatch(Command::CreateDatabase {
            path: scratch(name),
            format,
        });
        assert!(
            matches!(events.first(), Some(Event::DatabaseCreated { .. })),
            "{name} ({format}): {events:?}"
        );
    }
}

/// The template path applies the same rule against the template-defined
/// format (step 75.3, L43).
#[test]
fn template_creation_applies_the_same_suffix_rule() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::CreateDatabaseFromTemplate {
        path: scratch("contacts.sqlite"),
        template: "contacts".to_owned(),
    });
    match events.first() {
        Some(Event::Failed { code, reason }) => {
            assert_eq!(*code, FailureCode::InvalidInput, "{reason}");
            assert!(
                reason.contains(".sqlite") && reason.contains("Jet4"),
                "{reason}"
            );
        }
        other => panic!("expected a Failed event, got {other:?}"),
    }
}

/// Item M94 (rides this step): a capability refusal names the create-
/// database capability, not the relationship one.
#[test]
fn a_creation_refusal_carries_the_create_database_capability_code() {
    let mut caps = DriverCapabilities::full();
    caps.create_database = false;
    let mut port = MockPort::new();
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    for command in [
        Command::CreateDatabase {
            path: scratch("blocked.mdb"),
            format: DatabaseFormat::Jet4,
        },
        Command::CreateDatabaseFromTemplate {
            path: scratch("blocked-template.mdb"),
            template: "contacts".to_owned(),
        },
    ] {
        let events = core.dispatch(command);
        match events.first() {
            Some(Event::Failed { code, .. }) => assert_eq!(
                *code,
                FailureCode::Capability(DisabledReason::CreateDatabase),
                "{events:?}"
            ),
            other => panic!("expected a capability refusal, got {other:?}"),
        }
    }
}
