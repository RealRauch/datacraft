//! Index-leaf maintenance on row inserts (step 49.2, item M63).
//!
//! The engine resolves `MSysObjects` and `MSysRelationships` through their
//! physical index B-trees (evidence in `docs/FORMAT-JET.md`), so every row the
//! writer appends to an indexed table must grow the matching leaves. These
//! contracts drive the real writer over a fresh template file and read the
//! catalog row back through the pointer the new leaf entry carries.

use std::path::PathBuf;
use std::process::id;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_jet::StorageValue;
use datacraft_jet::jet::create::create_jet4_file;
use datacraft_jet::jet::data_page::DataPage;
use datacraft_jet::jet::index_pages::{KeyPart, RowPointer, decode_leaf, entry_pointer, leaf_entry};
use datacraft_jet::jet::{Jet4TableReader, JetTableDef, Jet4Writer, JetColumn, JetType};

const PAGE: usize = 4096;
const CATALOG: u32 = 2;
const RELATIONSHIPS: u32 = 5;

/// Unique scratch path: the test binary runs tests in parallel.
fn scratch(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("datacraft-jet-index-{}-{serial}-{name}", id()))
}

/// A brand-new file from the embedded engine template.
fn fresh() -> Vec<u8> {
    let path = scratch("fresh.mdb");
    let _ = std::fs::remove_file(&path);
    create_jet4_file(&path, 46_000.0).expect("fresh file");
    let bytes = std::fs::read(&path).expect("readable");
    let _ = std::fs::remove_file(&path);
    bytes
}

fn page(file: &[u8], number: u32) -> &[u8] {
    let at = number as usize * PAGE;
    &file[at..at + PAGE]
}

/// Decodes the row a leaf pointer addresses.
fn row_at(file: &[u8], def: &JetTableDef, pointer: RowPointer) -> Vec<StorageValue> {
    let data = page(file, pointer.page);
    let decoded = DataPage::decode(data).expect("data page decodes");
    let (start, end, _) = decoded.records[pointer.row as usize];
    datacraft_jet::jet::decode_row(&data[start..end], def, &[], PAGE).expect("row decodes")
}

fn value_by_column(def: &JetTableDef, values: &[StorageValue], name: &str) -> StorageValue {
    let position = def
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap_or_else(|| panic!("column {name} missing"));
    values[position].clone()
}

#[test]
fn creating_a_table_grows_both_catalog_leaves() {
    let mut writer = Jet4Writer::new(fresh()).expect("fresh writer");
    let columns = vec![
        JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
        JetColumn::new("name", JetType::Text, 0x02, 1, 0, 40),
    ];
    let tdef_page = writer.create_table("Contacts", &columns).expect("create");

    let file = writer.file();
    let parent_id_name = decode_leaf(7, page(file, 7)).expect("page 7");
    let id = decode_leaf(8, page(file, 8)).expect("page 8");
    assert_eq!(parent_id_name.entries.len(), 9, "one catalog row was added");
    assert_eq!(id.entries.len(), 9);
    assert_eq!(parent_id_name.parent, CATALOG);
    assert_eq!(id.parent, CATALOG);

    // The Id leaf gained the new table's TDEF page as its key.
    let expected_id_prefix = {
        let mut entry = leaf_entry(&[KeyPart::Integer(i64::from(tdef_page))], RowPointer { page: 0, row: 0 })
            .expect("id entry");
        entry.truncate(entry.len() - 4);
        entry
    };
    let id_entry = id
        .entries
        .iter()
        .find(|entry| entry.starts_with(&expected_id_prefix))
        .expect("Id entry for the new table");
    let pointer = entry_pointer(id_entry).expect("pointer");

    // That pointer addresses the catalog row the writer inserted.
    let catalog = JetTableDef::decode(file, CATALOG).expect("catalog decodes");
    let row = row_at(file, &catalog, pointer);
    assert_eq!(
        value_by_column(&catalog, &row, "Id"),
        StorageValue::Int(i64::from(tdef_page))
    );
    assert_eq!(
        value_by_column(&catalog, &row, "ParentId"),
        StorageValue::Int(0x0F00_0001)
    );
    assert_eq!(
        value_by_column(&catalog, &row, "Name"),
        StorageValue::Text("Contacts".into())
    );
    assert_eq!(
        value_by_column(&catalog, &row, "Type"),
        StorageValue::Int(1)
    );

    // The ParentIdName leaf carries the composite entry with the same pointer.
    let expected_composite = {
        let mut entry = leaf_entry(
            &[KeyPart::Integer(0x0F00_0001), KeyPart::Text("Contacts")],
            RowPointer { page: 0, row: 0 },
        )
        .expect("composite entry");
        entry.truncate(entry.len() - 4);
        entry
    };
    let composite = parent_id_name
        .entries
        .iter()
        .find(|entry| entry.starts_with(&expected_composite))
        .expect("ParentIdName entry for the new table");
    assert_eq!(
        entry_pointer(composite).expect("pointer"),
        pointer,
        "both leaves address the same catalog row"
    );
}

#[test]
fn inserted_rows_grow_the_relationship_leaves() {
    let mut writer = Jet4Writer::new(fresh()).expect("fresh writer");
    let def = JetTableDef::decode(writer.file(), RELATIONSHIPS).expect("MSysRelationships");
    let mut values = vec![StorageValue::Null; def.columns.len()];
    let mut set = |name: &str, value: StorageValue| {
        let position = def
            .columns
            .iter()
            .position(|column| column.name == name)
            .expect("column exists");
        values[position] = value;
    };
    set("szRelationship", StorageValue::Text("rel1".into()));
    set("grbit", StorageValue::Int(0));
    set("ccolumn", StorageValue::Int(1));
    set("icolumn", StorageValue::Int(1));
    set("szObject", StorageValue::Text("Child".into()));
    set("szColumn", StorageValue::Text("ParentId".into()));
    set("szReferencedObject", StorageValue::Text("Parent".into()));
    set("szReferencedColumn", StorageValue::Text("Id".into()));
    writer.insert_row(RELATIONSHIPS, &values).expect("insert");

    let file = writer.file();
    let expectations = [(14u32, "rel1"), (15, "Child"), (16, "Parent")];
    for (leaf_page, text) in expectations {
        let leaf = decode_leaf(leaf_page, page(file, leaf_page)).expect("relationship leaf");
        assert_eq!(leaf.parent, RELATIONSHIPS);
        assert_eq!(leaf.entries.len(), 1, "leaf {leaf_page} gained one row");
        let expected = {
            let mut entry = leaf_entry(
                &[KeyPart::Text(text)],
                RowPointer { page: 0, row: 0 },
            )
            .expect("entry");
            entry.truncate(entry.len() - 4);
            entry
        };
        assert!(
            leaf.entries[0].starts_with(&expected),
            "leaf {leaf_page} carries the {text:?} key"
        );
    }
}

#[test]
fn the_maintained_file_reopens_with_the_new_table() {
    let mut writer = Jet4Writer::new(fresh()).expect("fresh writer");
    let columns = vec![JetColumn::new(
        "name",
        JetType::Text,
        0x02,
        0,
        0,
        40,
    )];
    writer.create_table("People", &columns).expect("create");
    let path = scratch("reopen.mdb");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, writer.into_file()).expect("write");
    let bytes = std::fs::read(&path).expect("readable");

    let reader = Jet4TableReader::new(bytes).expect("reopens");
    let names: Vec<String> = reader
        .catalog()
        .expect("catalog")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(names.iter().any(|name| name == "People"), "{names:?}");
    drop(reader);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn tables_without_physical_indexes_leave_the_catalog_alone() {
    let mut writer = Jet4Writer::new(fresh()).expect("fresh writer");
    let columns = vec![JetColumn::new("v", JetType::LongInt, 0x03, 0, 0, 4)];
    let page_number = writer.create_table("Plain", &columns).expect("create");
    let before = writer.file().to_vec();
    writer
        .insert_row(page_number, &[StorageValue::Int(7)])
        .expect("insert");
    let file = writer.file();
    // The catalog leaves did not change; only the table's own data page did.
    assert_eq!(
        page(file, 7),
        page(&before, 7),
        "no catalog index entry for a user-table row"
    );
    assert_eq!(page(file, 8), page(&before, 8));
}
