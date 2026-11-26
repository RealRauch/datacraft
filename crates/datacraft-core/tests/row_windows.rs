//! Row counts for the windowed datasheet (phase 101, step 101.5; item
//! M132): the port answers a table's row count without reading a row.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Text { max_len: 10 }),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn ready_core(rows: usize) -> Core {
    let data: Vec<Vec<FieldValue>> = (0..rows)
        .map(|index| {
            vec![
                FieldValue::Text(format!("{index}")),
                FieldValue::Text(format!("name{index}")),
            ]
        })
        .collect();
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", data);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

#[test]
fn the_count_agrees_with_a_full_read() {
    let core = ready_core(25);
    assert_eq!(core.row_count("Customers"), Ok(25));
    assert_eq!(core.row_count("customers"), Ok(25), "case-insensitive");
}

#[test]
fn a_window_reads_the_requested_slice() {
    let core = ready_core(25);
    let window = core.read_row_window("Customers", 20, 10).expect("window");
    assert_eq!(window.len(), 5, "the window clamps at the end");
    assert_eq!(window[0][1], FieldValue::Text("name20".into()));
    let empty = core
        .read_row_window("Customers", 25, 10)
        .expect("a window at the end is empty");
    assert!(empty.is_empty());
}

#[test]
fn an_unknown_table_is_refused_precisely() {
    let core = ready_core(3);
    assert!(core.row_count("Missing").is_err());
    assert!(core.read_row_window("Missing", 0, 10).is_err());
}

#[test]
fn an_unopened_core_refuses_both_calls() {
    let core = Core::new(Box::new(MockPort::new()));
    assert!(core.row_count("Customers").is_err());
    assert!(core.read_row_window("Customers", 0, 10).is_err());
}
