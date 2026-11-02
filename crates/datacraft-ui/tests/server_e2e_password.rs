//! Encrypted Save-As and password changes over the JSON-RPC wire
//! (step 27.2, item M14).

mod support;

use datacraft_core::core::Core;
use datacraft_storage::detecting::DetectingPort;
use serde_json::json;

use support::{fixture_named, result_of, session};

#[test]
fn save_as_and_password_change_cross_the_wire() {
    let target =
        std::env::temp_dir().join(format!("datacraft-e2e-save-as-{}.mdb", std::process::id()));
    let _ = std::fs::remove_file(&target);

    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": fixture_named("passwordV2000.mdb").to_string_lossy(), "password":"password"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"SaveDatabaseAs":{"path": target.to_string_lossy()}}}),
        ],
    );
    let events = &result_of(&responses[1])["events"];
    assert!(
        events[0]["DatabaseSavedAs"]["encrypted"] == false,
        "{events}"
    );

    let mut plain = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut plain,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": target.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"inspect"}),
        ],
    );
    let events = &result_of(&responses[0])["events"];
    assert!(events[0].get("DatabaseOpened").is_some(), "{events}");
    let _ = std::fs::remove_file(&target);

    let change = std::env::temp_dir().join(format!(
        "datacraft-e2e-change-password-{}.mdb",
        std::process::id()
    ));
    std::fs::copy(fixture_named("passwordV2000-none.mdb"), &change).expect("copy the fixture");
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut core,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                   "params":{"OpenDatabase":{"path": change.to_string_lossy()}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"dispatch",
                   "params":{"ChangePassword":{"new_password":"wire"}}}),
        ],
    );
    let events = &result_of(&responses[1])["events"];
    assert!(
        events[0]["PasswordChanged"]["encrypted"] == true,
        "{events}"
    );

    let mut reopened = Core::new(Box::new(DetectingPort::new()));
    let responses = session(
        &mut reopened,
        &[json!({"jsonrpc":"2.0","id":1,"method":"dispatch",
                 "params":{"OpenDatabase":{"path": change.to_string_lossy(), "password":"wire"}}})],
    );
    let events = &result_of(&responses[0])["events"];
    assert!(events[0].get("DatabaseOpened").is_some(), "{events}");
    let _ = std::fs::remove_file(&change);
}
