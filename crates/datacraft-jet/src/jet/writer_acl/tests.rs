//! ACL-derivation contracts (step 49.2, item M63): the pure row lookups the
//! writer uses to grant a new object the engine's default access.

use super::{database_owner_in, workgroup_sid_in};
use crate::jet::tdef::{JetColumn, JetTableDef, JetType};
use crate::types::StorageValue;

/// Catalog shape: `Id` + `Owner` (+ filler) as the engine template carries.
fn catalog_def() -> JetTableDef {
    JetTableDef {
        table_type: 0x53,
        num_rows: 0,
        autonumber: 0,
        used_pages: 0,
        free_pages: 0,
        indexes: Vec::new(),
        index_section: Vec::new(),
        index_count: 0,
        real_index_count: 0,
        columns: vec![
            JetColumn::new("Name", JetType::Text, 0x02, 0, 0, 10),
            JetColumn::new("Id", JetType::LongInt, 0x03, 1, 0, 4),
            JetColumn::new("Owner", JetType::Binary, 0x02, 2, 0, 4),
        ],
    }
}

/// `MSysACEs` shape: `ObjectId` + `SID`.
fn aces_def() -> JetTableDef {
    JetTableDef {
        table_type: 0x53,
        num_rows: 0,
        autonumber: 0,
        used_pages: 0,
        free_pages: 0,
        indexes: Vec::new(),
        index_section: Vec::new(),
        index_count: 0,
        real_index_count: 0,
        columns: vec![
            JetColumn::new("ObjectId", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("SID", JetType::Binary, 0x02, 1, 0, 4),
        ],
    }
}

fn msysdb_row(owner: StorageValue) -> Vec<StorageValue> {
    vec![
        StorageValue::Text("MSysDb".into()),
        StorageValue::Int(0x1000_0000),
        owner,
    ]
}

#[test]
fn the_database_owner_is_the_msysdb_row_owner() {
    let def = catalog_def();
    let rows = vec![msysdb_row(StorageValue::Binary(vec![0xDB, 0x13]))];
    assert_eq!(
        database_owner_in(&def, &rows),
        Some(vec![0xDB, 0x13]),
        "the engine template's owner SID"
    );
}

#[test]
fn a_missing_or_empty_owner_yields_no_acl() {
    let def = catalog_def();
    assert_eq!(database_owner_in(&def, &[]), None, "no MSysDb row");
    assert_eq!(
        database_owner_in(&def, &[msysdb_row(StorageValue::Null)]),
        None,
        "Owner is NULL"
    );
    assert_eq!(
        database_owner_in(&def, &[msysdb_row(StorageValue::Binary(Vec::new()))]),
        None,
        "Owner is an empty binary"
    );

    // A definition without the Id or Owner columns cannot derive an owner.
    let mut stripped = catalog_def();
    stripped.columns.truncate(1);
    assert_eq!(
        database_owner_in(&stripped, &[msysdb_row(StorageValue::Binary(vec![1]))]),
        None
    );
}

#[test]
fn the_workgroup_sid_differs_from_the_owner() {
    let def = aces_def();
    let ace = |sid: StorageValue| vec![StorageValue::Int(0x1000_0000), sid];
    let rows = vec![
        ace(StorageValue::Binary(vec![0xDB, 0x13])), // the owner's own entry
        ace(StorageValue::Binary(Vec::new())),       // empty: skipped
        ace(StorageValue::Binary(vec![0xDA, 0x13])),
    ];
    assert_eq!(
        workgroup_sid_in(&def, &rows, &[0xDB, 0x13]),
        Some(vec![0xDA, 0x13])
    );
    assert_eq!(
        workgroup_sid_in(
            &def,
            &[ace(StorageValue::Binary(vec![0xDB, 0x13]))],
            &[0xDB, 0x13]
        ),
        None,
        "only the owner's own entry exists"
    );

    // A definition without the ObjectId or SID columns derives no grant.
    let mut stripped = aces_def();
    stripped.columns.truncate(1);
    assert_eq!(
        workgroup_sid_in(
            &stripped,
            &[ace(StorageValue::Binary(vec![1]))],
            &[0xDB, 0x13]
        ),
        None
    );
}
