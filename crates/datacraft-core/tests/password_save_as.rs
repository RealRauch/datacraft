//! Save-As and password-change dispatch (step 27.2, item M14).
//!
//! The commands are ordinary bus commands: capability-gated on the
//! driver's `passwords` surface, validated (an empty password is
//! ambiguous and refused), and observable through dedicated events. The
//! original database stays open and untouched. The real Jet4 round-trip
//! lives in the storage suite; this suite pins the core contract.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn core_with(caps: DriverCapabilities) -> Core {
    let table = TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut port = MockPort::with_tables(vec![table]);
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    assert!(
        matches!(events.first(), Some(Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    core
}

fn capable() -> Core {
    core_with(DriverCapabilities::full())
}

#[test]
fn save_as_writes_a_copy_and_keeps_the_database_open() {
    let mut core = capable();
    let path = PathBuf::from("copy.mdb");
    let events = core.dispatch(Command::SaveDatabaseAs {
        path: path.clone(),
        password: Some("secret".to_owned()),
    });
    assert!(
        matches!(
            events.first(),
            Some(Event::DatabaseSavedAs {
                path: saved,
                encrypted: true,
            }) if saved == &path
        ),
        "{events:?}"
    );
    assert!(
        core.inspect().database.is_some(),
        "save-as never closes the source"
    );
}

#[test]
fn save_as_without_a_password_reports_an_unencrypted_copy() {
    let mut core = capable();
    let events = core.dispatch(Command::SaveDatabaseAs {
        path: PathBuf::from("copy.mdb"),
        password: None,
    });
    assert!(
        matches!(
            events.first(),
            Some(Event::DatabaseSavedAs {
                encrypted: false,
                ..
            })
        ),
        "{events:?}"
    );
}

#[test]
fn change_password_is_an_ordinary_bus_command() {
    let mut core = capable();
    let events = core.dispatch(Command::ChangePassword {
        new_password: Some("secret".to_owned()),
    });
    assert!(
        matches!(
            events.first(),
            Some(Event::PasswordChanged { encrypted: true })
        ),
        "{events:?}"
    );
    let events = core.dispatch(Command::ChangePassword { new_password: None });
    assert!(
        matches!(
            events.first(),
            Some(Event::PasswordChanged { encrypted: false })
        ),
        "{events:?}"
    );
}

#[test]
fn both_commands_gate_on_the_passwords_capability() {
    let mut caps = DriverCapabilities::full();
    caps.passwords = false;
    let mut core = core_with(caps);

    let events = core.dispatch(Command::SaveDatabaseAs {
        path: PathBuf::from("copy.mdb"),
        password: None,
    });
    let reason = event_reason(&events);
    assert!(
        reason.contains("cannot save a copy with database passwords"),
        "{reason}"
    );

    let events = core.dispatch(Command::ChangePassword {
        new_password: Some("secret".to_owned()),
    });
    let reason = event_reason(&events);
    assert!(
        reason.contains("cannot change database passwords"),
        "{reason}"
    );
}

#[test]
fn an_empty_password_is_refused_precisely() {
    let mut core = capable();
    for command in [
        Command::SaveDatabaseAs {
            path: PathBuf::from("copy.mdb"),
            password: Some(String::new()),
        },
        Command::ChangePassword {
            new_password: Some(String::new()),
        },
    ] {
        let reason = event_reason(&core.dispatch(command));
        assert!(reason.contains("non-empty"), "{reason}");
    }
}

#[test]
fn both_commands_need_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    for command in [
        Command::SaveDatabaseAs {
            path: PathBuf::from("copy.mdb"),
            password: None,
        },
        Command::ChangePassword { new_password: None },
    ] {
        let reason = event_reason(&core.dispatch(command));
        assert!(reason.contains("no database"), "{reason}");
    }
}

fn event_reason(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a Failed event: {events:?}"))
}

#[test]
fn port_failures_surface_as_failed_events() {
    let mut port = MockPort::failing_writes("disk full");
    port.with_capabilities(DriverCapabilities::full());
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: None,
    });
    for command in [
        Command::SaveDatabaseAs {
            path: PathBuf::from("copy.mdb"),
            password: None,
        },
        Command::ChangePassword {
            new_password: Some("secret".to_owned()),
        },
    ] {
        let reason = event_reason(&core.dispatch(command));
        assert!(reason.contains("disk full"), "{reason}");
    }
}
