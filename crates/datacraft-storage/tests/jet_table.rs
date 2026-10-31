//! Table-definition, data-page and row decoding contract tests (step 2.3,
//! item H6). Fixtures are synthetic Jet4 files built byte-by-byte from the
//! public format documentation; real-file verification happens in step 2.4
//! via the CI oracle. Test-first: RED until the decoder modules exist.

use datacraft_storage::jet::{
    page_type, FileHeader, Jet4TableReader, JetColumn, JetTableDef, JetType, PAGE_TYPE_DATA,
    PAGE_TYPE_TDEF,
};
use datacraft_storage::{FormatVersion, StorageDataType, StorageValue};

const PAGE: usize = 4096;

/// Builds a valid Jet4 file shell (header page + `pages` zero pages).
fn file_shell(pages: usize) -> Vec<u8> {
    let mut file = vec![0u8; PAGE * (pages + 1)];
    file[0] = 0x00;
    file[1] = 0x01;
    file[2] = 0x00;
    file[3] = 0x00;
    file[4..20].copy_from_slice(b"Standard Jet DB\0");
    file[0x14..0x18].copy_from_slice(&0x0000_0001u32.to_le_bytes());
    file
}

/// Writes a Jet4 TDEF block at offset 8 of `page` (see `src/jet/tdef.rs`).
#[allow(clippy::too_many_arguments)]
fn write_tdef(
    file: &mut [u8],
    tdef_page: usize,
    num_rows: u32,
    table_type: u8,
    columns: &[JetColumn],
) {
    let mut block: Vec<u8> = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&0u32.to_le_bytes()); // unknown
    block.extend_from_slice(&num_rows.to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x01); // autonum flag
    block.extend_from_slice(&[0u8; 3]);
    block.extend_from_slice(&0u32.to_le_bytes()); // complex autonumber
    block.extend_from_slice(&[0u8; 8]); // unknown
    block.push(table_type);
    block.extend_from_slice(&columns.len().to_le_bytes()); // max_cols
    let var_cols = columns.iter().filter(|c| !c.is_fixed()).count() as u16;
    block.extend_from_slice(&var_cols.to_le_bytes());
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // used_pages
    block.extend_from_slice(&0u32.to_le_bytes()); // free_pages
    for col in columns {
        block.push(col.data_type as u8);
        block.extend_from_slice(&0u32.to_le_bytes()); // unknown
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&col.var_index.unwrap_or(0).to_le_bytes());
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes()); // misc (sort order)
        block.extend_from_slice(&0u16.to_le_bytes()); // misc_ext
        block.push(col.flags);
        block.push(0); // misc_flags: uncompressed unicode
        block.extend_from_slice(&0u32.to_le_bytes()); // unknown
        block.extend_from_slice(&col.fixed_offset.to_le_bytes());
        block.extend_from_slice(&col.len.to_le_bytes());
    }
    for col in columns {
        let name: Vec<u8> = col.name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        block.extend_from_slice(&(col.name.chars().count() as u16).to_le_bytes());
        block.extend_from_slice(&name);
    }
    let len = block.len() as u32 - 4; // tdef_len covers everything after itself
    block[..4].copy_from_slice(&len.to_le_bytes());

    let base = tdef_page * PAGE;
    file[base] = PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    file[base + 8..8 + block.len()].copy_from_slice(&block);
}

/// Writes a Jet4 data page with the given raw rows (stored back to front).
fn write_data_page(file: &mut [u8], page: usize, tdef_page: u32, rows: &[Vec<u8>]) {
    let base = page * PAGE;
    file[base] = PAGE_TYPE_DATA;
    file[base + 1] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&tdef_page.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&(rows.len() as u16).to_le_bytes());
    let mut offset = PAGE;
    for (i, row) in rows.iter().enumerate() {
        offset -= row.len();
        file[base + offset..base + offset + row.len()].copy_from_slice(row);
        file[base + 14 + i * 2..base + 16 + i * 2].copy_from_slice(&(offset as u16).to_le_bytes());
    }
}

/// Builds a Jet4 row: fixed values, variable values, null mask.
/// `null_mask_bits` holds one bool per column: `true` = non-null (or bit
/// value for boolean columns).
fn build_row(
    num_cols: u16,
    fixed: &[u8],
    var_values: &[&str],
    null_mask_bits: &[bool],
) -> Vec<u8> {
    let mut row: Vec<u8> = Vec::new();
    row.extend_from_slice(&num_cols.to_le_bytes());
    row.extend_from_slice(fixed);
    for v in var_values {
        row.extend_from_slice(&v.encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>());
    }
    let eod = row.len() as u16;
    row.extend_from_slice(&eod.to_le_bytes());
    // var_table: reverse order — entry 0 is the offset of the LAST var column.
    let mut starts: Vec<u16> = Vec::new();
    let mut pos = 2u16 + fixed.len() as u16;
    for v in var_values {
        starts.push(pos);
        pos += (v.encode_utf16().count() * 2) as u16;
    }
    for start in starts.iter().rev() {
        row.extend_from_slice(&start.to_le_bytes());
    }
    row.extend_from_slice(&(var_values.len() as u16).to_le_bytes());
    let mask_len = (num_cols as usize + 7) / 8;
    let mut mask = vec![0u8; mask_len];
    for (i, non_null) in null_mask_bits.iter().enumerate() {
        if *non_null {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    row.extend_from_slice(&mask);
    row
}

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
        def.columns[1].storage_type(),
        StorageDataType::Text
    );
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
    assert_eq!(table.schema.name, "Customers");
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
fn currency_float_datetime_decode_from_fixed_columns() {
    let columns = vec![
        JetColumn::new("Money", JetType::Currency, 0x01, 0, 0, 8),
        JetColumn::new("Ratio", JetType::Double, 0x01, 0, 8, 8),
        JetColumn::new("When", JetType::DateTime, 0x01, 0, 16, 8),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut fixed = Vec::new();
    fixed.extend_from_slice(&123_4500i64.to_le_bytes());
    fixed.extend_from_slice(&2.5f64.to_le_bytes());
    fixed.extend_from_slice(&44_000.5f64.to_le_bytes());
    let row = build_row(3, &fixed, &[], &[true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][0], StorageValue::Currency(123_4500));
    assert_eq!(table.rows[0][1], StorageValue::Float(2.5));
    assert_eq!(table.rows[0][2], StorageValue::DateTime(44_000.5));
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

    // Inline memo: flag 0x8000_0000, data right after the length dword.
    let mut inline_var: Vec<u8> = Vec::new();
    inline_var.extend_from_slice(&(0x8000_0000u32 | 5).to_le_bytes());
    inline_var.extend_from_slice(b"hello");
    // Single-LVAL memo: flag 0x4000_0000, record pointer (page 4, row 0).
    let mut lval_var: Vec<u8> = Vec::new();
    lval_var.extend_from_slice(&(0x4000_0000u32 | 7).to_le_bytes());
    lval_var.extend_from_slice(&((4u32 << 8) | 0).to_le_bytes());
    lval_var.extend_from_slice(&0u32.to_le_bytes());
    // Chained memo: no flag, first record pointer (page 5, row 0).
    let mut chain_var: Vec<u8> = Vec::new();
    chain_var.extend_from_slice(&2u32.to_le_bytes());
    chain_var.extend_from_slice(&((5u32 << 8) | 0).to_le_bytes());
    chain_var.extend_from_slice(&0u32.to_le_bytes());

    let row1 = build_row(2, &1i32.to_le_bytes(), &[&String::from_utf8_lossy(&inline_var)], &[true, true]);
    let row2 = build_row(2, &2i32.to_le_bytes(), &[&String::from_utf8_lossy(&lval_var)], &[true, true]);
    let row3 = build_row(2, &3i32.to_le_bytes(), &[&String::from_utf8_lossy(&chain_var)], &[true, true]);
    // Rows carry raw var bytes — rebuild them to avoid UTF-16 string encoding.
    let row1 = raw_var_row(2, &1i32.to_le_bytes(), &inline_var, &[true, true]);
    let row2 = raw_var_row(2, &2i32.to_le_bytes(), &lval_var, &[true, true]);
    let row3 = raw_var_row(2, &3i32.to_le_bytes(), &chain_var, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row1, row2, row3]);

    // LVAL page 4: type-1 record "world" (single record).
    write_data_page(&mut file, 4, 0, &[b"world".to_vec()]);
    // LVAL page 5: type-2 chain: [next ptr][data]; next = (5,1), then terminator.
    let mut chunk1: Vec<u8> = Vec::new();
    chunk1.extend_from_slice(&((5u32 << 8) | 1).to_le_bytes());
    chunk1.extend_from_slice(b"foo-");
    let chunk2: Vec<u8> = b"bar".to_vec();
    write_data_page(&mut file, 5, 0, &[chunk1, chunk2]);

    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text("hello".into()));
    assert_eq!(table.rows[1][1], StorageValue::Text("world".into()));
    assert_eq!(table.rows[2][1], StorageValue::Text("foo-bar".into()));
}

/// Like `build_row` but writes the var section as raw bytes (no UTF-16
/// encoding) — used for memo headers, which are binary structures.
fn raw_var_row(num_cols: u16, fixed: &[u8], var: &[u8], null_mask_bits: &[bool]) -> Vec<u8> {
    let mut row: Vec<u8> = Vec::new();
    row.extend_from_slice(&num_cols.to_le_bytes());
    row.extend_from_slice(fixed);
    row.extend_from_slice(var);
    let eod = row.len() as u16;
    row.extend_from_slice(&eod.to_le_bytes());
    row.extend_from_slice(&((2u16 + fixed.len() as u16)).to_le_bytes()); // var_table[0], reverse order
    row.extend_from_slice(&1u16.to_le_bytes()); // var_len
    let mask_len = (num_cols as usize + 7) / 8;
    let mut mask = vec![0u8; mask_len];
    for (i, non_null) in null_mask_bits.iter().enumerate() {
        if *non_null {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    row.extend_from_slice(&mask);
    row
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
fn overflow_rows_are_rejected_precisely() {
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers_columns());
    let row = build_row(3, &1i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let base = 2 * PAGE;
    let off0 = u16::from_le_bytes([file[base + 14], file[base + 15]]);
    let flagged = off0 | 0x4000;
    file[base + 14..base + 16].copy_from_slice(&flagged.to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("overflow must be rejected");
    assert!(matches!(err, datacraft_storage::StorageError::Unsupported { .. }));
}

#[test]
fn reader_rejects_non_jet4_files() {
    let mut file = file_shell(1);
    file[0x14..0x18].copy_from_slice(&0x0000_0000u32.to_le_bytes()); // Jet3
    assert!(matches!(
        Jet4TableReader::new(file),
        Err(FormatCheck::Jet3Detected) | Err(FormatCheck::Other(_))
    ));
}

/// Local error shaping for the reader constructor test.
use datacraft_storage::StorageError as FormatCheck;

#[test]
fn file_header_still_decodes() {
    let file = file_shell(1);
    let header = FileHeader::decode(&file, file.len() as u64).expect("header");
    assert_eq!(header.format, FormatVersion::Jet4);
}
