//! RunSql command and SQL window contract (step 13.3, item H37).
//!
//! Agents and the UI share the same command: `RunSql` materializes the
//! open database through the port, runs one SELECT and reports the result;
//! `Core::run_sql` is the data seam the UI renders from.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{
    AutoKind, FieldDef, FieldKind, FieldValue, TableDef,
};

fn table() -> TableDef {
    TableDef::new(
        "customers",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
        ],
    )
}

fn core() -> Core {
    let mut port = MockPort::with_tables(vec![table()]);
    port.with_row_data(
        "customers",
        vec![
            vec![FieldValue::Integer(1), FieldValue::Text("Ada".into())],
            vec![FieldValue::Integer(2), FieldValue::Text("Grace".into())],
        ],
    );
    let mut core = Core::new(Box::new(port));
    let events = core.dispatch(Command::OpenDatabase {
        path: "memory.mock".into(),
    });
    assert!(matches!(events[0], Event::DatabaseOpened { .. }), "{events:?}");
    core
}

#[test]
fn run_sql_reports_the_result_row_count() {
    let mut core = core();
    let events = core.dispatch(Command::RunSql {
        sql: "SELECT name FROM customers ORDER BY name".into(),
    });
    assert_eq!(events, vec![Event::SqlRan { rows: 2 }]);
}

#[test]
fn core_run_sql_returns_columns_and_values_for_the_ui() {
    let core = core();
    let result = core
        .run_sql("SELECT id, name FROM customers WHERE id = 1")
        .expect("SQL runs");
    assert_eq!(result.columns, ["id", "name"]);
    assert_eq!(
        result.rows,
        vec![vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into())
        ]]
    );
}

#[test]
fn rejected_statements_become_precise_failed_events() {
    let mut core = core();
    let events = core.dispatch(Command::RunSql {
        sql: "DELETE FROM customers".into(),
    });
    match &events[0] {
        Event::Failed { reason } => assert!(reason.contains("only SELECT"), "{reason}"),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn sql_requires_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let events = core.dispatch(Command::RunSql {
        sql: "SELECT 1".into(),
    });
    match &events[0] {
        Event::Failed { reason } => assert!(reason.contains("no database"), "{reason}"),
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(core.run_sql("SELECT 1").is_err());
}

#[test]
fn the_sql_window_toggles_through_the_bus() {
    let mut core = core();
    assert!(!core.inspect().sql_window_open);
    let events = core.dispatch(Command::ToggleSqlWindow);
    assert_eq!(events, vec![Event::SqlWindowToggled { open: true }]);
    assert!(core.inspect().sql_window_open);
    core.dispatch(Command::ToggleSqlWindow);
    assert!(!core.inspect().sql_window_open);
}

#[test]
fn run_sql_commands_round_trip_through_json() {
    let command = Command::RunSql {
        sql: "SELECT 1".into(),
    };
    let json = serde_json::to_string(&command).expect("serialize command");
    let back: Command = serde_json::from_str(&json).expect("deserialize command");
    assert_eq!(back, command);
}
