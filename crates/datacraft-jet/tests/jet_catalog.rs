//! Catalog (MSysObjects) enumeration contract tests (step 2.4, item H6/H7).
//!
//! Strategy per the public evidence base (Project-Copperfin #5541): a table's
//! `Id` alone is NOT a reliable TDEF pointer (page reuse stales it) — names
//! attach only when the Id lands on an independently scanned TDEF page.
//! Test-first: RED until `catalog()`/`open_table()` exist.

use datacraft_jet::StorageError;
use datacraft_jet::jet::{Jet4TableReader, JetColumn, JetTableDef, JetType};

mod support;

use support::{build_row, file_shell, ucs2, write_data_page, write_tdef};

/// Builds the catalog table (MSysObjects stand-in): Id/Type/Flags fixed,
/// Name variable — laid out at page 2 per the format convention.
fn catalog_columns() -> Vec<JetColumn> {
    vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
        JetColumn::new("Type", JetType::LongInt, 0x01, 2, 4, 4),
        JetColumn::new("Flags", JetType::LongInt, 0x01, 3, 8, 4),
    ]
}

fn user_table() -> JetTableDef {
    JetTableDef {
        table_type: 0x4E,
        num_rows: 1,
        autonumber: 0,
        columns: vec![
            JetColumn::new("Key", JetType::LongInt, 0x01, 0, 0, 4),
            JetColumn::new("Title", JetType::Text, 0x02, 1, 0, 255),
        ],
        used_pages: 0,
        free_pages: 0,
        indexes: Vec::new(),
        index_section: Vec::new(),
        index_count: 0,
        real_index_count: 0,
    }
}

fn catalog_file() -> Vec<u8> {
    let mut file = file_shell(6);
    // Page 2: catalog TDEF; page 3: one catalog row pointing at page 4.
    write_tdef(&mut file, 2, 1, 0x53, &catalog_columns());
    // Fixed columns Id/Type/Flags, variable Name — assembled manually.
    let row = catalog_row(4, "Customers", 1, 0);
    write_data_page(&mut file, 3, 2, &[row]);

    // Page 4: user table TDEF; page 5: its data.
    let def = user_table();
    write_tdef(&mut file, 4, 1, 0x4E, &def.columns);
    let user_row = build_row(2, &42i32.to_le_bytes(), &["Hello"], &[true, true]);
    write_data_page(&mut file, 5, 4, &[user_row]);
    file
}

/// Assembles a catalog row: Id/Type/Flags fixed (offsets 0/4/8), Name variable.
fn catalog_row(id: u32, name: &str, object_type: i32, flags: i32) -> Vec<u8> {
    let mut row: Vec<u8> = Vec::new();
    row.extend_from_slice(&4u16.to_le_bytes()); // num_cols
    row.extend_from_slice(&id.to_le_bytes()); // fixed: Id @0
    row.extend_from_slice(&object_type.to_le_bytes()); // Type @4
    row.extend_from_slice(&flags.to_le_bytes()); // Flags @8
    row.extend_from_slice(&ucs2(name)); // Name (variable)
    let eod = row.len() as u16;
    row.extend_from_slice(&eod.to_le_bytes());
    row.extend_from_slice(&14u16.to_le_bytes()); // var_table[0]: Name starts at 2+12
    row.extend_from_slice(&1u16.to_le_bytes()); // var_len
    row.push(0x0F); // null mask: all four present
    row
}

#[test]
fn catalog_enumerates_user_tables_with_resolved_pages() {
    let reader = Jet4TableReader::new(catalog_file()).expect("reader");
    let entries = reader.catalog().expect("catalog");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "Customers");
    assert_eq!(entries[0].object_type, 1);
    assert_eq!(entries[0].tdef_page, Some(4));
    assert_eq!(reader.table_names().expect("names"), ["Customers"]);
}

#[test]
fn stale_catalog_ids_do_not_attach_names() {
    let mut file = catalog_file();
    // Second catalog row with an Id pointing at no TDEF page (stale reuse).
    let stale = catalog_row(99, "Ghost", 1, 0);
    write_data_page(&mut file, 3, 2, &[catalog_row(4, "Customers", 1, 0), stale]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let entries = reader.catalog().expect("catalog");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].name, "Ghost");
    assert_eq!(entries[1].tdef_page, None);
    assert_eq!(reader.table_names().expect("names"), ["Customers"]);
}

#[test]
fn nameless_catalog_rows_are_skipped() {
    let mut file = catalog_file();
    // Row with NULL Name (mask bit cleared) — not a nameable object.
    let mut null_name = catalog_row(4, "", 1, 0);
    let mask_at = null_name.len() - 1;
    null_name[mask_at] &= !0x02; // clear the Name presence bit
    write_data_page(
        &mut file,
        3,
        2,
        &[catalog_row(4, "Customers", 1, 0), null_name],
    );
    let reader = Jet4TableReader::new(file).expect("reader");
    assert_eq!(reader.table_names().expect("names"), ["Customers"]);
}

#[test]
fn open_table_resolves_name_and_overrides_schema_label() {
    let reader = Jet4TableReader::new(catalog_file()).expect("reader");
    let table = reader.open_table("Customers").expect("table");
    assert_eq!(table.schema.name, "Customers");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0][1], StorageValue::Text("Hello".into()));
    let err = reader
        .open_table("Missing")
        .expect_err("unknown table must fail");
    assert!(matches!(err, StorageError::TableNotFound(ref t) if t == "Missing"));
}

use datacraft_jet::StorageValue;
