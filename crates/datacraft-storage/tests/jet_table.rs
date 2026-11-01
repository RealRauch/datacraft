//! Table-definition, data-page and row decoding contract tests (step 2.3,
//! item H6). Fixtures are synthetic Jet4 files built byte-by-byte from the
//! public format documentation (`docs/FORMAT-JET.md`); real-file verification
//! happens in step 2.4 via the CI oracle.

use datacraft_storage::jet::{
    FileHeader, Jet4TableReader, JetColumn, JetTableDef, JetType, PAGE_TYPE_TDEF, decode_row,
    page_type,
};
use datacraft_storage::{FormatVersion, StorageDataType, StorageError, StorageValue};

mod support;

use support::{PAGE, build_row, file_shell, raw_var_row, ucs2, write_data_page, write_tdef};

fn customers_columns() -> Vec<JetColumn> {
    vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
        JetColumn::new("Active", JetType::Bool, 0x01, 2, 0, 0),
    ]
}

#[test]
fn tdef_decodes_table_and_columns() {
    let mut file = file_shell(2);
    write_tdef(&mut file, 1, 2, 0x4E, &customers_columns());
    let def = JetTableDef::decode(&file, 1).expect("tdef decodes");
    assert_eq!(def.table_type, 0x4E);
    assert_eq!(def.num_rows, 2);
    assert_eq!(def.columns.len(), 3);
    assert_eq!(def.columns[0].name, "Id");
    assert_eq!(def.columns[0].data_type, JetType::LongInt);
    assert!(def.columns[0].is_fixed());
    assert_eq!(def.columns[1].name, "Name");
    assert_eq!(def.columns[1].data_type, JetType::Text);
    assert!(!def.columns[1].is_fixed());
    assert_eq!(
        def.columns[1].data_type.storage_type(),
        StorageDataType::Text
    );
    assert_eq!(def.schema_columns()[1].name, "Name");
}

#[test]
fn page_type_reads_first_byte() {
    let mut file = file_shell(2);
    write_tdef(&mut file, 1, 0, 0x4E, &customers_columns());
    assert_eq!(page_type(&file[PAGE..2 * PAGE]), PAGE_TYPE_TDEF);
    assert_eq!(page_type(&file[0..PAGE]), 0x00); // header page
}

#[test]
fn rows_decode_fixed_var_null_and_bit() {
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 2, 0x4E, &customers_columns());
    let row1 = build_row(3, &1i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    let row2 = build_row(3, &2i32.to_le_bytes(), &[], &[true, false, false]);
    write_data_page(&mut file, 2, 1, &[row1, row2]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    // Real table names come from the catalog (step 2.4); the reader derives
    // a stable label from the TDEF page.
    assert_eq!(table.schema.name, "tdef_0001");
    assert_eq!(table.schema.columns[1].data_type, StorageDataType::Text);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(
        table.rows[0],
        vec![
            StorageValue::Int(1),
            StorageValue::Text("Ada".into()),
            StorageValue::Bit(true)
        ]
    );
    assert_eq!(
        table.rows[1],
        vec![
            StorageValue::Int(2),
            StorageValue::Null,
            StorageValue::Bit(false)
        ]
    );
}

#[test]
fn currency_float_datetime_and_numeric_decode_from_fixed_columns() {
    let columns = vec![
        JetColumn::new("Money", JetType::Currency, 0x01, 0, 0, 8),
        JetColumn::new("Ratio", JetType::Double, 0x01, 0, 8, 8),
        JetColumn::new("When", JetType::DateTime, 0x01, 0, 16, 8),
        JetColumn::new("Exact", JetType::Numeric, 0x01, 0, 24, 17),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut fixed = Vec::new();
    fixed.extend_from_slice(&123_4500i64.to_le_bytes());
    fixed.extend_from_slice(&2.5f64.to_le_bytes());
    fixed.extend_from_slice(&44_000.5f64.to_le_bytes());
    // Numeric: sign byte 0x80 (positive) + big-endian 96-bit magnitude.
    let mut numeric = vec![0x80u8];
    numeric.extend_from_slice(&[0u8; 14]);
    numeric.extend_from_slice(&1234u16.to_be_bytes());
    fixed.extend_from_slice(&numeric);
    let row = build_row(4, &fixed, &[], &[true, true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][0], StorageValue::Currency(123_4500));
    assert_eq!(table.rows[0][1], StorageValue::Float(2.5));
    assert_eq!(table.rows[0][2], StorageValue::DateTime(44_000.5));
    assert_eq!(table.rows[0][3], StorageValue::Decimal(1234));
}

#[test]
fn memo_inline_single_and_chained_lval_decode() {
    // Table with one memo column; three rows exercise all storage forms.
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    let mut file = file_shell(6);
    write_tdef(&mut file, 1, 3, 0x4E, &columns);

    // Inline memo: flag 0x8000_0000, UCS-2LE payload after the length dword.
    let mut inline_var: Vec<u8> = Vec::new();
    inline_var.extend_from_slice(&(0x8000_0000u32 | 10).to_le_bytes());
    inline_var.extend_from_slice(&ucs2("hello"));
    // Single-LVAL memo: flag 0x4000_0000, record pointer (page 4, row 0).
    let mut lval_var: Vec<u8> = Vec::new();
    lval_var.extend_from_slice(&(0x4000_0000u32 | 10).to_le_bytes());
    lval_var.extend_from_slice(&(4u32 << 8).to_le_bytes());
    lval_var.extend_from_slice(&0u32.to_le_bytes());
    // Chained memo: no flag, 14 bytes total, first record pointer (page 5, row 0).
    let mut chain_var: Vec<u8> = Vec::new();
    chain_var.extend_from_slice(&14u32.to_le_bytes());
    chain_var.extend_from_slice(&(5u32 << 8).to_le_bytes());
    chain_var.extend_from_slice(&0u32.to_le_bytes());

    let row1 = raw_var_row(2, &1i32.to_le_bytes(), &inline_var, &[true, true]);
    let row2 = raw_var_row(2, &2i32.to_le_bytes(), &lval_var, &[true, true]);
    let row3 = raw_var_row(2, &3i32.to_le_bytes(), &chain_var, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row1, row2, row3]);

    // LVAL page 4: type-1 record "world" (single record, UCS-2LE).
    write_data_page(&mut file, 4, 0, &[ucs2("world")]);
    // LVAL page 5: type-2 chain: [next ptr][data]; next = (5,1), then terminator.
    let mut chunk1: Vec<u8> = Vec::new();
    chunk1.extend_from_slice(&((5u32 << 8) | 1).to_le_bytes());
    chunk1.extend_from_slice(&ucs2("foo-"));
    let mut chunk2: Vec<u8> = Vec::new();
    chunk2.extend_from_slice(&0u32.to_le_bytes()); // chain terminator
    chunk2.extend_from_slice(&ucs2("bar"));
    write_data_page(&mut file, 5, 0, &[chunk1, chunk2]);

    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text("hello".into()));
    assert_eq!(table.rows[1][1], StorageValue::Text("world".into()));
    assert_eq!(table.rows[2][1], StorageValue::Text("foo-bar".into()));
}

#[test]
fn deleted_rows_are_skipped() {
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers_columns());
    let row = build_row(3, &1i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    // Mark record 0 as deleted (0x8000 flag on its offset).
    let base = 2 * PAGE;
    let off0 = u16::from_le_bytes([file[base + 14], file[base + 15]]);
    let flagged = off0 | 0x8000;
    file[base + 14..base + 16].copy_from_slice(&flagged.to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows.len(), 0);
}

#[test]
fn overflow_rows_resolve_through_the_row_pointer() {
    // Record 0 on page 2 is an overflow slot: its 4 bytes point at page 3
    // row 0, which carries the actual row bytes.
    let mut file = file_shell(4);
    write_tdef(&mut file, 1, 1, 0x4E, &customers_columns());
    let row = build_row(3, &1i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    write_data_page(&mut file, 3, 1, &[row]);
    // Real files flag the overflow target with 0x8000 (skipped in iteration).
    let target_base = 3 * PAGE;
    let t_off = u16::from_le_bytes([file[target_base + 14], file[target_base + 15]]);
    file[target_base + 14..target_base + 16].copy_from_slice(&(t_off | 0x8000).to_le_bytes());
    // Page 2: overflow slot whose pointer names (page 3, row 0).
    let base = 2 * PAGE;
    file[base] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&1u32.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
    let slot = 3u32 << 8;
    file[base + 14..base + 16].copy_from_slice(&(0x4000u16 | slot as u16 & 0x1FFF).to_le_bytes());
    file[base + 768..base + 772].copy_from_slice(&slot.to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("overflow resolves");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0][1], StorageValue::Text("Ada".into()));
}

#[test]
fn reader_accepts_jet3_files_since_l1() {
    // A minimal Jet3 shell: 2048-byte pages, an unprotected password field
    // (empty plaintext XOR the fixed key) and the fixed-key RC4 header
    // region every real file carries (step 16.2 / M5).
    let mut file = vec![0u8; 2 * 2048];
    file[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    file[4..20].copy_from_slice(b"Standard Jet DB\0");
    file[0x14..0x18].copy_from_slice(&0x0000_0000u32.to_le_bytes());
    datacraft_storage::jet::crypto::rc4(
        &datacraft_storage::jet::crypto::HEADER_RC4_KEY,
        &mut file[0x18..0x18 + datacraft_storage::jet::crypto::JET3_HEADER_REGION],
    );
    let reader = Jet4TableReader::new(file).expect("Jet3 opens since step 10.6 (L1)");
    assert_eq!(reader.header().format, FormatVersion::Jet3);
    assert_eq!(reader.header().page_size, 2048);
}

#[test]
fn file_header_still_decodes() {
    let file = file_shell(1);
    let header = FileHeader::decode(&file, file.len() as u64).expect("header");
    assert_eq!(header.format, FormatVersion::Jet4);
}

#[test]
fn all_jet_types_map_from_byte_and_storage() {
    let cases = [
        (0x01u8, JetType::Bool, StorageDataType::Bit),
        (0x02, JetType::Byte, StorageDataType::Int1),
        (0x03, JetType::Int, StorageDataType::Int2),
        (0x04, JetType::LongInt, StorageDataType::Int4),
        (0x05, JetType::Currency, StorageDataType::Currency),
        (0x06, JetType::Float, StorageDataType::Float4),
        (0x07, JetType::Double, StorageDataType::Float8),
        (0x08, JetType::DateTime, StorageDataType::DateTime),
        (0x09, JetType::Binary, StorageDataType::Binary),
        (0x0A, JetType::Text, StorageDataType::Text),
        (0x0B, JetType::Ole, StorageDataType::Binary),
        (0x0C, JetType::Memo, StorageDataType::Memo),
        (0x0F, JetType::Guid, StorageDataType::Guid),
        (0x10, JetType::Numeric, StorageDataType::Decimal),
        (0x12, JetType::Complex, StorageDataType::Complex),
    ];
    for (byte, expected, storage) in cases {
        assert_eq!(JetType::from_byte(byte).expect("known type"), expected);
        assert_eq!(expected.storage_type(), storage);
        assert_eq!(expected as u8, byte);
    }
}

#[test]
fn schema_evolution_rows_report_added_columns_as_null() {
    // Nick (fixed) and Tag (variable) were added after the row was written:
    // num_cols=1 means only Id is stored.
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Extra", JetType::LongInt, 0x01, 3, 4, 4),
        JetColumn::new("Tag", JetType::Text, 0x02, 2, 0, 255),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let row = build_row(1, &9i32.to_le_bytes(), &[], &[true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(
        table.rows[0],
        vec![StorageValue::Int(9), StorageValue::Null, StorageValue::Null]
    );
}

#[test]
fn tdef_with_indexes_and_usage_maps_skips_cleanly() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
    ];
    let mut file = file_shell(2);
    support::write_tdef_with_indexes(&mut file, 1, 5, &columns);
    let def = JetTableDef::decode(&file, 1).expect("tdef with indexes decodes");
    assert_eq!(def.columns.len(), 2);
    assert_eq!(def.columns[1].name, "Name");
    assert_eq!(def.num_rows, 5);
}

#[test]
fn negative_numeric_and_hostile_var_ranges_fail_cleanly() {
    let columns = vec![
        JetColumn::new("Exact", JetType::Numeric, 0x01, 0, 0, 17),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut fixed = vec![0x00u8]; // negative sign
    fixed.extend_from_slice(&[0u8; 14]);
    fixed.extend_from_slice(&5u16.to_be_bytes());
    let mut row = raw_var_row(2, &fixed, &ucs2("Ada"), &[true, true]);
    write_data_page(&mut file, 2, 1, &[row.clone()]);
    let reader = Jet4TableReader::new(file.clone()).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][0], StorageValue::Decimal(-5));

    // Corrupt the variable start offset beyond the row: precise error.
    let table_at = row.len() - 1 - 2 - 2; // mask(1) + var_len(2) before end
    row[table_at..table_at + 2].copy_from_slice(&4000u16.to_le_bytes());
    let def = JetTableDef::decode(&file, 1).expect("tdef");
    let err = decode_row(&row, &def, &file, PAGE).expect_err("hostile offset must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("invalid range")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn two_variable_columns_use_adjacent_boundaries() {
    let mut first = JetColumn::new("First", JetType::Text, 0x02, 1, 0, 255);
    first.var_index = Some(0);
    let mut second = JetColumn::new("Second", JetType::Text, 0x02, 2, 0, 255);
    second.var_index = Some(1);
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        first,
        second,
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let row = build_row(3, &1i32.to_le_bytes(), &["ab", "c"], &[true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text("ab".into()));
    assert_eq!(table.rows[0][2], StorageValue::Text("c".into()));
}

#[test]
fn var_column_present_in_schema_but_missing_in_row_is_null() {
    // Mask marks Name non-null, but the row carries zero variable fields
    // (column added later) — the boundary reports Null, not a crash.
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    // build_row with no variable values: var_len = 0 while the mask marks
    // Name non-null — the missing variable column reports Null.
    let row = build_row(2, &1i32.to_le_bytes(), &[], &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Null);
}

#[test]
fn ole_column_resolves_to_binary() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Doc", JetType::Ole, 0x02, 1, 0, 0),
    ];
    let mut file = file_shell(4);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut var: Vec<u8> = Vec::new();
    var.extend_from_slice(&(0x4000_0000u32 | 4).to_le_bytes());
    var.extend_from_slice(&(3u32 << 8).to_le_bytes());
    var.extend_from_slice(&0u32.to_le_bytes());
    let row = raw_var_row(2, &1i32.to_le_bytes(), &var, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    write_data_page(&mut file, 3, 0, &[b"PK\x03\x04".to_vec()]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(
        table.rows[0][1],
        StorageValue::Binary(b"PK\x03\x04".to_vec())
    );
}

#[test]
fn compressed_text_without_marker_and_with_big_chars() {
    let mut col = JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255);
    col.compressed_text = true;
    let columns = vec![JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4), col];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    // Without the FF FE marker the payload is plain UCS-2LE even on a
    // compressed-flag column; the Zhe pair exercises the high-byte path.
    let body = ucs2("Ada\u{0416}");
    // Name descriptor misc_flags: block sits at the page end; block_len =
    // 55 + 2*25 + names(6+10) + usage(10+2) for Id/Name.
    let name_desc = PAGE + 8 + 55 + 25;
    file[name_desc + 16] = 0x01; // misc_flags: compressed unicode
    let row = raw_var_row(2, &1i32.to_le_bytes(), &body, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text("Ada\u{0416}".into()));
}

#[test]
fn reader_exposes_header() {
    let file = file_shell(1);
    let reader = Jet4TableReader::new(file).expect("reader");
    assert_eq!(reader.header().page_size, 4096);
}

#[test]
fn byte_int_float_fixed_columns_decode() {
    let columns = vec![
        JetColumn::new("B", JetType::Byte, 0x01, 0, 0, 1),
        JetColumn::new("I", JetType::Int, 0x01, 0, 1, 2),
        JetColumn::new("F", JetType::Float, 0x01, 0, 3, 4),
        JetColumn::new("L", JetType::LongInt, 0x01, 0, 7, 4),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut fixed = Vec::new();
    fixed.push(0xFFu8); // Byte: -1
    fixed.extend_from_slice(&(-2i16).to_le_bytes());
    fixed.extend_from_slice(&1.5f32.to_le_bytes());
    fixed.extend_from_slice(&(-3i32).to_le_bytes());
    let row = build_row(4, &fixed, &[], &[true, true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][0], StorageValue::Int(255)); // Jet Byte is unsigned
    assert_eq!(table.rows[0][1], StorageValue::Int(-2));
    assert_eq!(table.rows[0][2], StorageValue::Float(1.5));
    assert_eq!(table.rows[0][3], StorageValue::Int(-3));
}

#[test]
fn empty_text_variable_slot_yields_empty_string() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    // One variable slot with zero length (start == end) — empty text.
    let row = raw_var_row(2, &1i32.to_le_bytes(), &[], &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text(String::new()));
}

#[test]
fn overclaimed_var_entries_read_adjacent_bytes_structurally() {
    // A row claiming more variable entries than it carries stays in bounds
    // (the reverse walk reads adjacent mask bytes) and yields structurally
    // valid values — hostile rows never panic, per the boundary contract.
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers_columns());
    let def = JetTableDef::decode(&file, 1).expect("tdef");
    let mut row = raw_var_row(3, &1i32.to_le_bytes(), &ucs2("Ada"), &[true, true, true]);
    let vt_at = row.len() - 1 - 2 - 2;
    row[vt_at..vt_at + 2].copy_from_slice(&2u16.to_le_bytes()); // claim 2 entries
    let values = decode_row(&row, &def, &file, PAGE).expect("structurally decodable");
    assert_eq!(values.len(), 3);
}

/// Column type 0x11 (BACKLOG L2 decision): a fixed-storage long binary —
/// the fixture's MSysAccessObjects rows embed OLE compound documents
/// (the `D0 CF 11 E0 A1 B1 1A E1` signature, verified on the fixture).
#[test]
fn column_type_0x11_decodes_as_fixed_long_binary() {
    let columns = vec![JetColumn::new("Data", JetType::LongBinary, 0x03, 0, 0, 16)];
    let mut file = file_shell(2);
    write_tdef(&mut file, 1, 1, 0x53, &columns);
    let def = JetTableDef::decode(&file, 1).expect("0x11 decodes since the L2 decision");
    assert_eq!(def.columns[0].data_type, JetType::LongBinary);
    assert_eq!(
        def.columns[0].data_type.storage_type(),
        StorageDataType::Binary
    );

    // A row carrying the OLE compound-file signature decodes to Binary.
    let mut blob = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    blob.extend_from_slice(&[0u8; 8]);
    let row = build_row(1, &blob, &[], &[true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("opens");
    let data = reader.read_table(1).expect("reads");
    assert_eq!(
        data.rows[0][0],
        StorageValue::Binary({
            let mut b = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
            b.extend_from_slice(&[0u8; 8]);
            b
        })
    );
}
