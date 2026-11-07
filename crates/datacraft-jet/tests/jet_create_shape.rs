//! Engine-accepted create-path shape (step 49.2, item M63 acceptance pins).
//!
//! Every assertion below mirrors byte facts the DAO oracle verified on
//! 2026-11-07 (file accepted, `TableDefs` enumerates, rows read and written
//! through the engine). They are the guard rails for the create path: a
//! regression in any of them makes the engine refuse the table again.

use std::path::PathBuf;
use std::process::id;

use datacraft_jet::StorageValue;
use datacraft_jet::jet::create::create_jet4_file;
use datacraft_jet::jet::data_page::DataPage;
use datacraft_jet::jet::{Jet4TableReader, Jet4Writer, JetColumn, JetTableDef, JetType};

const PAGE: usize = 4096;
const CATALOG: u32 = 2;
const ACES: u32 = 3;

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("datacraft-create-shape-{}-{name}", id()))
}

/// A fresh file with one created table (autonumber + text), one row inserted.
fn built(name: &str) -> (Vec<u8>, u32) {
    let path = scratch(name);
    let _ = std::fs::remove_file(&path);
    create_jet4_file(&path, 46_000.0).expect("fresh");
    let bytes = std::fs::read(&path).expect("readable");
    let _ = std::fs::remove_file(&path);
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x07, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 80),
    ];
    let page = writer.create_table("Contacts", &columns).expect("create");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("Ada".into())],
        )
        .expect("insert");
    (writer.into_file(), page)
}

fn page(bytes: &[u8], number: u32) -> &[u8] {
    let at = number as usize * PAGE;
    &bytes[at..at + PAGE]
}

#[test]
fn the_definition_ends_with_the_engine_trailer() {
    let (file, tdef) = built("trailer.mdb");
    let block = &page(&file, tdef)[8..];
    let len = u32::from_le_bytes(block[0..4].try_into().expect("4 bytes")) as usize;
    assert!(len >= 8, "the definition has a trailer");
    assert_eq!(
        &block[len - 8..len],
        &[0u8; 8],
        "the engine's eight-byte TDEF trailer"
    );
    // The AutoNumber column sets the head flag; without one it stays zero
    // (observed engine output).
    assert_eq!(block[0x10], 0x01, "autonumber flag (head offset 0x10)");
}

#[test]
fn the_usage_page_follows_the_definition() {
    let (file, tdef) = built("order.mdb");
    let def = JetTableDef::decode(&file, tdef).expect("definition");
    let usage = def.used_pages & 0x00FF_FFFF;
    let usage_page = usage >> 8;
    assert!(
        usage_page > tdef,
        "the engine allocates the TDEF before its usage page ({tdef} < {usage_page})"
    );
    let decoded = DataPage::decode(page(&file, usage_page)).expect("usage page");
    assert_eq!(decoded.records.len(), 2, "used + free map rows");
    for (start, end, _) in decoded.records {
        assert_eq!(
            end - start,
            69,
            "a fresh usage row carries a 64-byte bitmap (512-page window)"
        );
    }
}

#[test]
fn creating_a_table_grants_the_engine_default_acl() {
    let (file, tdef) = built("acl.mdb");
    let reader = Jet4TableReader::new(file.clone()).expect("reader");
    let aces = reader.read_table(ACES).expect("MSysACEs rows").rows;
    let mut grants: Vec<(Vec<u8>, i64)> = aces
        .iter()
        .filter_map(|row| match row.as_slice() {
            [
                StorageValue::Int(object),
                StorageValue::Binary(sid),
                StorageValue::Int(mask),
                _,
            ] if *object == i64::from(tdef) => Some((sid.clone(), *mask)),
            _ => None,
        })
        .collect();
    grants.sort();
    assert_eq!(
        grants,
        [(vec![0xDA, 0x13], 1_048_319), (vec![0xDB, 0x13], 983_294)],
        "the engine's default full-control grants for a new object"
    );

    // The catalog row carries the database owner's SID (observed: the
    // template's MSysDb owner), not a placeholder.
    let catalog = JetTableDef::decode(&file, CATALOG).expect("catalog");
    let name = catalog
        .columns
        .iter()
        .position(|column| column.name == "Name")
        .expect("Name");
    let owner = catalog
        .columns
        .iter()
        .position(|column| column.name == "Owner")
        .expect("Owner");
    let rows = reader.read_table(CATALOG).expect("catalog rows").rows;
    let row = rows
        .iter()
        .find(|row| row.get(name) == Some(&StorageValue::Text("Contacts".into())))
        .expect("the new catalog row");
    assert_eq!(
        row.get(owner),
        Some(&StorageValue::Binary(vec![0xDB, 0x13])),
        "owner SID of the database"
    );
}

#[test]
fn the_created_table_reopens_with_its_row() {
    let (file, tdef) = built("reopen.mdb");
    let reader = Jet4TableReader::new(file).expect("reader");
    let names: Vec<String> = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(names.iter().any(|name| name == "Contacts"), "{names:?}");
    let rows = reader.read_table(tdef).expect("rows").rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], StorageValue::Int(1));
}
