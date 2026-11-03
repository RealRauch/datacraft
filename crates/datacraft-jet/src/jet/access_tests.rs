//! Unit tests for the `MSysAccessStorage` tree and macro decoder.

use super::*;
use crate::types::{ColumnSchema, StorageDataType, TableSchema};

fn entry(id: i64, parent_id: i64, entry_type: i64, name: &str, data: &[u8]) -> AccessStorageEntry {
    AccessStorageEntry {
        id,
        parent_id,
        entry_type,
        name: name.to_owned(),
        data: data.to_vec(),
    }
}

fn storage(entries: Vec<AccessStorageEntry>) -> AccessStorage {
    AccessStorage { entries }
}

#[test]
fn dir_data_stops_at_malformed_or_truncated_entries() {
    // Header only: no entries.
    assert!(decode_dir_data(&[0, 0, 0, 0]).is_empty());
    // Truncated name length.
    assert!(decode_dir_data(&[0, 0, 0, 0, 4, 0x40, b'A', 0]).is_empty());
    // Wrong tag: no entry is decoded.
    assert!(decode_dir_data(&[0, 0, 0, 0, 5, 0, 0, 0]).is_empty());
    // Odd name byte count cannot decode.
    let mut odd = vec![0, 0, 0, 0, 4, 5];
    odd.extend_from_slice(&[b'A', 0, 0, 0, 0]);
    assert!(decode_dir_data(&odd).is_empty());
}

#[test]
fn dir_data_decodes_names_and_indices() {
    let mut data = vec![0, 0, 0, 0];
    for (index, name) in [(0u32, "Macro"), (7, "Other")] {
        let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        data.push(4);
        data.push((units.len() + 4) as u8);
        data.extend_from_slice(&units);
        data.extend_from_slice(&index.to_le_bytes());
    }
    assert_eq!(
        decode_dir_data(&data),
        vec![(0, "Macro".to_owned()), (7, "Other".to_owned())]
    );
}

#[test]
fn argument_scan_skips_noise_and_unterminated_runs() {
    let long = "  A\0B\0CD\0Delete * from [T]\0";
    let bytes: Vec<u8> = long.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(decode_arguments(&bytes), vec!["Delete * from [T]"]);
    // Unterminated printable text and a short run produce nothing.
    let unterminated: Vec<u8> = "abcdef".encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert!(decode_arguments(&unterminated).is_empty());
    let short: Vec<u8> = "ab\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert!(decode_arguments(&short).is_empty());
}

#[test]
fn argument_scan_strips_expression_type_markers() {
    // Access prefixes expression-form arguments with value-type markers
    // (`<` string literal, `$`, `&`, `"`, `,`) and may carry a record tag
    // byte in front of the marker (observed on the self-created
    // programmability fixture). The remainder starts with a letter or a
    // bracketed name — stored argument text never begins with these
    // markers themselves.
    let long = "'<Inventory sample ready.\0[StockLevel] > 0\0$Category Picker\0&Items In Stock\0";
    let bytes: Vec<u8> = long.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(
        decode_arguments(&bytes),
        vec![
            "Inventory sample ready.".to_owned(),
            "[StockLevel] > 0".to_owned(),
            "Category Picker".to_owned(),
            "Items In Stock".to_owned(),
        ]
    );
    // A marker followed by anything but a letter, a bracket or another
    // marker is part of the text and stays untouched.
    let untouched: Vec<u8> = "< 2 and 2 make 4\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(decode_arguments(&untouched), vec!["< 2 and 2 make 4"]);
}

#[test]
fn macro_arguments_need_a_scripts_storage_and_a_blob() {
    let without_scripts = storage(vec![entry(1, 1, 1, "Root", &[])]);
    assert!(without_scripts.macro_arguments("AutoExec").is_none());

    let mut dir = vec![0, 0, 0, 0];
    let name: Vec<u8> = "AutoExec"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    dir.push(4);
    dir.push((name.len() + 4) as u8);
    dir.extend_from_slice(&name);
    dir.extend_from_slice(&0u32.to_le_bytes());
    let no_blob = storage(vec![
        entry(7, 1, 1, "Scripts", &[]),
        entry(8, 7, 2, "\u{3}DirData", &dir),
        entry(9, 7, 1, "0", &[]),
    ]);
    assert!(no_blob.macro_arguments("AutoExec").is_none());
    assert!(no_blob.macro_arguments("Missing").is_none());

    // Scripts node of the wrong kind answers no definition.
    let stream_scripts = storage(vec![entry(7, 1, 2, "Scripts", &[])]);
    assert!(stream_scripts.macro_arguments("AutoExec").is_none());

    // No directory stream means no name mapping.
    let no_dir = storage(vec![
        entry(7, 1, 1, "Scripts", &[]),
        entry(9, 7, 1, "0", &[]),
    ]);
    assert!(no_dir.macro_arguments("AutoExec").is_none());

    // The blob search descends nested storages.
    let blob: Vec<u8> = "Delete * from [T]\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let nested = storage(vec![
        entry(7, 1, 1, "Scripts", &[]),
        entry(8, 7, 2, "\u{3}DirData", &dir),
        entry(9, 7, 1, "0", &[]),
        entry(10, 9, 1, "Inner", &[]),
        entry(11, 10, 2, "Blob", &blob),
    ]);
    assert_eq!(
        nested.macro_arguments("AutoExec"),
        Some(vec!["Delete * from [T]".to_owned()])
    );
}

fn tree_table(columns: &[&str], rows: Vec<Vec<StorageValue>>) -> JetTableData {
    let columns = columns
        .iter()
        .map(|name| ColumnSchema::new(*name, StorageDataType::Int4, true))
        .collect();
    JetTableData {
        schema: TableSchema::new("MSysAccessStorage", columns),
        rows,
    }
}

#[test]
fn a_tree_table_missing_a_column_is_a_precise_corrupt_error() {
    let table = tree_table(&["Id"], Vec::new());
    let error = AccessStorage::from_table(&table).expect_err("missing columns fail");
    assert!(
        matches!(error, StorageError::Corrupt { ref detail, .. } if detail.contains("ParentId")),
        "{error:?}"
    );
}

#[test]
fn unaddressable_rows_are_skipped_and_stream_bytes_are_kept() {
    let table = tree_table(
        &["Id", "ParentId", "Type", "Name", "Lv"],
        vec![
            vec![
                StorageValue::Null,
                StorageValue::Int(1),
                StorageValue::Int(1),
                StorageValue::Text("Skipped".to_owned()),
                StorageValue::Null,
            ],
            vec![
                StorageValue::Int(1),
                StorageValue::Int(1),
                StorageValue::Int(1),
                StorageValue::Text("Root".to_owned()),
                StorageValue::Null,
            ],
            vec![
                StorageValue::Int(2),
                StorageValue::Int(1),
                StorageValue::Int(2),
                StorageValue::Text("Blob".to_owned()),
                StorageValue::Binary(vec![1, 2, 3]),
            ],
        ],
    );
    let tree = AccessStorage::from_table(&table).expect("valid rows project");
    assert!(tree.entry("Skipped").is_none());
    assert_eq!(tree.children(1).len(), 2);
    assert_eq!(tree.entry("Blob").expect("stream").data, vec![1, 2, 3]);
}

#[test]
fn stored_macro_marks_name_only_entries() {
    let decoded = stored_macro(
        "AutoExec".to_owned(),
        Some(vec!["Startup Screen".to_owned()]),
    );
    assert!(decoded.definition_decoded);
    assert_eq!(decoded.arguments, vec!["Startup Screen"]);
    let name_only = stored_macro("Legacy".to_owned(), None);
    assert!(!name_only.definition_decoded);
    assert!(name_only.arguments.is_empty());
}
