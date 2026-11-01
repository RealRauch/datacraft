//! Contract suite for the QBE query model (step 7.2, item H26).
//!
//! Decision default 7: in-memory predicates, no SQL parser. The QBE grid
//! edits `QueryDef`s through `SetQbeCell`; every identifier that reaches
//! the engine passes the schema whitelist (LESSONS #5). Queries persist
//! in the sidecar document (additive v1 field).

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::qbe::{QbeRow, QbeValue, QueryDef, SortOrder};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, SchemaDocument, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn customers_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn opened_core() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// creation
// ---------------------------------------------------------------------------

#[test]
fn create_query_emits_and_lands_in_the_snapshot() {
    let mut core = opened_core();
    assert_eq!(
        core.dispatch(Command::CreateQuery {
            name: "Active".into()
        }),
        vec![Event::QueryCreated {
            name: "Active".into()
        }]
    );
    let db = core.inspect().database.expect("open");
    assert_eq!(db.queries.len(), 1);
    assert_eq!(db.queries[0].name, "Active");
    assert_eq!(db.queries[0].table, "");
}

#[test]
fn duplicate_query_names_are_rejected() {
    let mut core = opened_core();
    core.dispatch(Command::CreateQuery {
        name: "Active".into(),
    });
    let reason = failed_reason(&core.dispatch(Command::CreateQuery {
        name: "active".into(),
    }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn create_requires_an_open_database() {
    let reason = failed_reason(&Core::new(Box::new(MockPort::new())).dispatch(
        Command::CreateQuery {
            name: "Active".into(),
        },
    ));
    assert!(reason.contains("no database"), "{reason}");
}

// ---------------------------------------------------------------------------
// QBE cell edits
// ---------------------------------------------------------------------------

fn active_query(core: &mut Core) {
    core.dispatch(Command::CreateQuery {
        name: "Active".into(),
    });
}

#[test]
fn the_table_cell_is_whitelisted_against_tables() {
    let mut core = opened_core();
    active_query(&mut core);
    assert_eq!(
        core.dispatch(Command::SetQbeCell {
            query: "Active".into(),
            row: QbeRow::Table,
            col: 0,
            value: QbeValue::Text("Customers".into()),
        }),
        vec![Event::QbeCellSet {
            query: "Active".into()
        }]
    );
    assert_eq!(
        core.inspect().database.unwrap().queries[0].table,
        "Customers"
    );

    let reason = failed_reason(&core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Ghost".into()),
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn field_cells_are_whitelisted_and_extend_the_grid() {
    let mut core = opened_core();
    active_query(&mut core);
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    // Column 0 first mention creates the column; second mention renames.
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("name".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 1,
        value: QbeValue::Text("id".into()),
    });
    let fields = &core.inspect().database.unwrap().queries[0].fields;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "name");

    let reason = failed_reason(&core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 2,
        value: QbeValue::Text("ghost".into()),
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn duplicate_query_columns_are_rejected() {
    let mut core = opened_core();
    active_query(&mut core);
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("name".into()),
    });
    let reason = failed_reason(&core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 1,
        value: QbeValue::Text("NAME".into()),
    }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn show_cells_toggle_and_sort_cells_parse() {
    let mut core = opened_core();
    active_query(&mut core);
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("name".into()),
    });

    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Show,
        col: 0,
        value: QbeValue::Bool(false),
    });
    assert!(!core.inspect().database.unwrap().queries[0].fields[0].shown);

    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Sort,
        col: 0,
        value: QbeValue::Text("ascending".into()),
    });
    assert_eq!(
        core.inspect().database.unwrap().queries[0].fields[0].sort,
        SortOrder::Ascending
    );
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Sort,
        col: 0,
        value: QbeValue::Text("descending".into()),
    });
    assert_eq!(
        core.inspect().database.unwrap().queries[0].fields[0].sort,
        SortOrder::Descending
    );
    // Empty resets to unsorted.
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Sort,
        col: 0,
        value: QbeValue::Text(String::new()),
    });
    assert_eq!(
        core.inspect().database.unwrap().queries[0].fields[0].sort,
        SortOrder::None
    );

    let reason = failed_reason(&core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Sort,
        col: 0,
        value: QbeValue::Text("sideways".into()),
    }));
    assert!(reason.to_lowercase().contains("sort"), "{reason}");
}

#[test]
fn criteria_cells_append_and_edit() {
    let mut core = opened_core();
    active_query(&mut core);
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("name".into()),
    });
    // First criteria on a column creates the "or" row.
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Criteria(0),
        col: 0,
        value: QbeValue::Text("> 5".into()),
    });
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Criteria(1),
        col: 0,
        value: QbeValue::Text("< 2".into()),
    });
    let criteria = core.inspect().database.unwrap().queries[0].fields[0]
        .criteria
        .clone();
    assert_eq!(criteria, ["> 5", "< 2"]);
    // Re-editing an existing row overwrites it.
    core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Criteria(0),
        col: 0,
        value: QbeValue::Text("= 7".into()),
    });
    let criteria = core.inspect().database.unwrap().queries[0].fields[0]
        .criteria
        .clone();
    assert_eq!(criteria, ["= 7", "< 2"]);
    // Empty criteria are rejected (use a narrower grid instead).
    let reason = failed_reason(&core.dispatch(Command::SetQbeCell {
        query: "Active".into(),
        row: QbeRow::Criteria(2),
        col: 0,
        value: QbeValue::Text(String::new()),
    }));
    assert!(reason.contains("criteria"), "{reason}");
}

// ---------------------------------------------------------------------------
// queries in tabs and the sidecar
// ---------------------------------------------------------------------------

#[test]
fn query_objects_open_as_tabs() {
    let mut core = opened_core();
    core.dispatch(Command::CreateQuery {
        name: "Active".into(),
    });
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Query,
        "Active",
    )));
    assert!(
        matches!(
            events.as_slice(),
            [Event::TabOpened { .. }, Event::ActiveTabChanged { .. }]
        ),
        "{events:?}"
    );
}

#[test]
fn deleting_a_query_closes_its_tabs() {
    let mut core = opened_core();
    core.dispatch(Command::CreateQuery {
        name: "Active".into(),
    });
    core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Query,
        "Active",
    )));
    assert_eq!(
        core.dispatch(Command::DeleteQuery {
            name: "Active".into()
        }),
        vec![
            Event::TabClosed {
                tab_id: datacraft_core::TabId::new(1)
            },
            Event::QueryDropped {
                name: "Active".into()
            },
        ]
    );
}

#[test]
fn the_sidecar_persists_queries() {
    let mut query = QueryDef::new("Active", "Customers");
    query.fields.push(datacraft_core::qbe::QbeField {
        name: "name".into(),
        shown: true,
        sort: SortOrder::None,
        criteria: vec!["> 5".into()],
    });

    let mut doc = SchemaDocument::new("shop");
    doc.queries.push(query.clone());
    let back = SchemaDocument::from_json(&doc.to_json()).unwrap();
    assert_eq!(back.queries, vec![query]);

    // v1 documents without the queries field still load (additive default).
    let legacy = SchemaDocument::new("shop").to_json().replace(
        "\"database_name\": \"shop\"",
        "\"database_name\": \"shop\",\n  \"queries\": []",
    );
    let _ = SchemaDocument::from_json(&legacy).expect("explicit empty queries parse");
}
