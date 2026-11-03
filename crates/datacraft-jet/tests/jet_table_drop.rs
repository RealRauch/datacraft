//! Jet table-drop contract (step 24.2, item M7).
//!
//! Dropping a table removes its catalog row and TDEF/TDAT pages, returns
//! every freed page to the global free map and leaves the remaining tables
//! readable across a reopen. Failures leave the file untouched.

use std::path::{Path, PathBuf};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_jet::{
    StorageValue,
    jet::{Jet4TableReader, Jet4Writer, JetColumn, JetType},
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

fn fixture_copy(stem: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("datacraft-drop-{}-{stem}.mdb", std::process::id()));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

fn scratch_table() -> TableDef {
    TableDef::new(
        "DropMe",
        vec![FieldDef::new("note", FieldKind::Text { max_len: 20 })],
    )
}

#[test]
fn the_writer_frees_the_table_pages_and_its_catalog_row() {
    let path = fixture_copy("writer");
    let file = std::fs::read(&path).expect("copy readable");
    let before = Jet4TableReader::new(file.clone()).expect("reader opens");
    let untouched = before
        .table_names()
        .expect("catalog")
        .into_iter()
        .find(|name| before.open_table(name).is_ok())
        .expect("a readable fixture table");

    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "DropWriter",
            &[JetColumn::new("note", JetType::Text, 0x02, 0, 0, 40)],
        )
        .expect("table created");
    writer
        .insert_row(tdef, &[StorageValue::Text("row".into())])
        .expect("row inserted");
    let data_pages: Vec<u32> = writer.data_pages_of(tdef).collect();
    assert!(!data_pages.is_empty(), "the insert allocated a data page");
    writer.drop_table(tdef).expect("table dropped");
    let written = writer.into_file();

    let reader = Jet4TableReader::new(written.clone()).expect("written file opens");
    let names = reader.table_names().expect("catalog after drop");
    assert!(!names.iter().any(|name| name == "DropWriter"), "{names:?}");
    let other = reader
        .open_table(&untouched)
        .expect("other table still reads");
    assert!(!other.rows.is_empty(), "{untouched} keeps its rows");

    let check = Jet4Writer::new(written).expect("written file is a valid writer input");
    let global = check.global_free_map().expect("global map decodes");
    for page in data_pages {
        assert!(
            global.owns_free(page),
            "freed data page {page} is marked free globally"
        );
    }
    assert!(
        !check.tdef_pages().contains(&tdef),
        "the TDEF page no longer heads a table"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_port_drop_is_visible_across_a_reopen() {
    let path = fixture_copy("port");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    port.create_table(&scratch_table()).expect("create");
    port.insert_row("DropMe", &[FieldValue::Text("row".to_owned())])
        .expect("insert");
    port.drop_table("DropMe").expect("drop");
    assert!(!port.tables().iter().any(|table| table.name == "DropMe"));
    let error = port
        .read_rows("DropMe", RowRange { start: 0, len: 10 })
        .expect_err("the table is gone");
    assert!(matches!(error, PortError::TableNotFound(_)), "{error:?}");
    port.close();

    port.open(&path, None).expect("reopen");
    assert!(!port.tables().iter().any(|table| table.name == "DropMe"));
    assert!(
        !port.tables().is_empty(),
        "the pre-existing catalog survives the drop"
    );
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn dropping_an_unknown_table_is_precise_and_leaves_the_file_intact() {
    let path = fixture_copy("missing");
    let before = std::fs::read(&path).expect("copy readable");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    let error = port.drop_table("NoSuchTable").expect_err("unknown table");
    assert!(matches!(error, PortError::TableNotFound(_)), "{error:?}");
    port.close();
    let after = std::fs::read(&path).expect("copy readable");
    assert_eq!(before, after, "a failed drop leaves the file untouched");
    let _ = std::fs::remove_file(&path);
}
