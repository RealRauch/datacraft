//! CreateDatabase command contract (phase 29, step 29.1, item H44).
//!
//! Creating a database file is a bus command like every user-visible
//! action: exactly one command, one `DatabaseCreated` event, independent
//! of the open-database state. RED state: `Command::CreateDatabase` and
//! `Event::DatabaseCreated` do not exist yet — the suite fails to compile
//! until the command lands.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, MockPort};

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
