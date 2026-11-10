//! Database-password command flow (step 16.2, item M5).
//!
//! The password travels on `OpenDatabase`; missing/wrong passwords surface
//! as dedicated events so every client (UI prompt, CLI flag, MCP parameter)
//! can react without parsing prose.

use std::path::Path;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;

fn core_with_password(password: &str) -> Core {
    let mut port = MockPort::new();
    port.with_password(password);
    Core::new(Box::new(port))
}

#[test]
fn a_password_required_open_emits_the_prompt_event() {
    let mut core = core_with_password("secret");
    let events = core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    assert!(
        matches!(&events[..], [Event::PasswordRequired { path }] if path == Path::new("shop.mdb")),
        "{events:?}"
    );
    assert!(core.inspect().database.is_none());
}

#[test]
fn a_wrong_password_emits_the_rejection_event() {
    let mut core = core_with_password("secret");
    let events = core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: Some("wrong".into()),
        source: None,
    });
    assert!(
        matches!(&events[..], [Event::PasswordRejected { path }] if path == Path::new("shop.mdb")),
        "{events:?}"
    );
    assert!(core.inspect().database.is_none());
}

#[test]
fn the_correct_password_opens_the_database() {
    let mut core = core_with_password("secret");
    let events = core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: Some("secret".into()),
        source: None,
    });
    assert!(
        matches!(&events[..], [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    assert!(core.inspect().database.is_some());
}

#[test]
fn open_json_keeps_the_password_optional() {
    let without: Command =
        serde_json::from_str(r#"{"OpenDatabase":{"path":"shop.mdb"}}"#).expect("old JSON");
    assert!(matches!(
        without,
        Command::OpenDatabase { password: None, .. }
    ));
    let with: Command =
        serde_json::from_str(r#"{"OpenDatabase":{"path":"shop.mdb","password":"secret"}}"#)
            .expect("password JSON");
    assert!(matches!(
        with,
        Command::OpenDatabase {
            password: Some(password),
            ..
        } if password == "secret"
    ));
}
