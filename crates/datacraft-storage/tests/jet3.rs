//! Jet 3 (Access 97) read-support contract tests (step 10.6, item L1).
//!
//! Facts per `docs/FORMAT-JET.md` "Jet 3 (Access 97)" (public-doc source:
//! MDB Tools HACKING): 2048-byte pages, 10-byte data page headers, a
//! 35-byte TDEF header with 18-byte column descriptors, 1-byte name
//! lengths with codepage (CP1252) text, 1-byte row counters and offsets.
//! Rows whose data exceeds 255 bytes need the jump-table mechanism and
//! are a precise `Unsupported` for now (documented L1 remainder).

use datacraft_storage::jet::{
    FileHeader, Jet4TableReader, JetColumn, JetTableDef, JetType, decode_row_jet3,
};
use datacraft_storage::{FormatVersion, StorageError, StorageValue};

const PAGE: usize = 2048;

/// Jet3 file shell: signature, product string, version dword 0.
fn shell(pages: usize) -> Vec<u8> {
    let mut file = vec![0u8; PAGE * (pages + 1)];
    file[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    file[4..20].copy_from_slice(b"Standard Jet DB\0");
    file[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
    file
}

/// Writes a Jet3 TDEF page: `'VC'` header word, 35-byte block, 18-byte
/// descriptors, 1-byte-length CP1252 names.
fn write_tdef(file: &mut [u8], page: usize, columns: &[(JetColumn, Vec<u8>)]) {
    let base = page * PAGE;
    file[base] = 0x02;
    file[base + 1] = 0x01;
    file[base + 2..base + 4].copy_from_slice(b"VC");
    let mut block: Vec<u8> = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&0u32.to_le_bytes()); // num_rows
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x4E); // user table
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes()); // max_cols
    let var_cols = columns.iter().filter(|(c, _)| !c.is_fixed()).count() as u16;
    block.extend_from_slice(&var_cols.to_le_bytes());
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // used_pages
    block.extend_from_slice(&0u32.to_le_bytes()); // free_pages
    for (col, _) in columns {
        block.push(col.data_type as u8);
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes()); // offset_V
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&0x0409u16.to_le_bytes()); // sort order
        let misc: u16 = if col.data_type == JetType::Text {
            0x04E4
        } else {
            0
        };
        block.extend_from_slice(&misc.to_le_bytes()); // codepage for text
        block.extend_from_slice(&0u16.to_le_bytes()); // unknown
        block.push(col.flags);
        block.extend_from_slice(&col.fixed_offset.to_le_bytes());
        block.extend_from_slice(&col.len.to_le_bytes());
    }
    for (_, name) in columns {
        block.push(name.len() as u8);
        block.extend_from_slice(name);
    }
    block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // var-col pointer terminator
    let len = block.len() as u32;
    block[..4].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
}

/// Builds one Jet3 row: 1-byte counters and offsets, empty slots for NULL
/// variable columns, the null mask last.
fn jet3_row(num_cols: u8, fixed: &[u8], var_cols: &[&[u8]], present: &[bool]) -> Vec<u8> {
    let mut row = vec![num_cols];
    row.extend_from_slice(fixed);
    let mut starts = Vec::new();
    for bytes in var_cols {
        starts.push(row.len() as u8);
        row.extend_from_slice(bytes);
    }
    row.push(row.len() as u8); // eod (data length so far)
    for start in starts.iter().rev() {
        row.push(*start);
    }
    row.push(var_cols.len() as u8);
    let mut mask = vec![0u8; (num_cols as usize).div_ceil(8)];
    for (i, present) in present.iter().enumerate() {
        if *present {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    row.extend_from_slice(&mask);
    row
}

fn write_data_page(file: &mut [u8], page: usize, tdef: u32, rows: &[Vec<u8>]) {
    let base = page * PAGE;
    file[base] = 0x01;
    file[base + 1] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&tdef.to_le_bytes());
    file[base + 8..base + 10].copy_from_slice(&(rows.len() as u16).to_le_bytes());
    let mut end = PAGE;
    for (i, row) in rows.iter().enumerate() {
        let start = end - row.len();
        file[base + start..base + start + row.len()].copy_from_slice(row);
        file[base + 10 + i * 2..base + 12 + i * 2].copy_from_slice(&(start as u16).to_le_bytes());
        end = start;
    }
    let free = end - (10 + 2 * rows.len());
    file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
}

/// The last row's raw bytes (the fixture pages hold exactly one row).
fn last_row_bytes(file: &[u8], page: usize, row_len: usize) -> Vec<u8> {
    let base = page * PAGE;
    file[base + PAGE - row_len..base + PAGE].to_vec()
}

fn catalog_columns() -> Vec<(JetColumn, Vec<u8>)> {
    vec![
        (
            JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
            b"Id".to_vec(),
        ),
        (
            JetColumn::new("Name", JetType::Text, 0x02, 1, 4, 100),
            vec![b'G', b'r', 0xF6, 0xDF, b'e'], // "Größe" in CP1252
        ),
        (
            JetColumn::new("Active", JetType::Bool, 0x01, 2, 0, 0),
            b"Active".to_vec(),
        ),
    ]
}

#[test]
fn jet3_headers_derive_2048_byte_pages() {
    let file = shell(2);
    let header = FileHeader::decode(&file, file.len() as u64).expect("decodes");
    assert_eq!(header.format, FormatVersion::Jet3);
    assert_eq!(header.page_size, 2048);
}

#[test]
fn jet3_tdef_decodes_descriptors_and_cp1252_names() {
    let mut file = shell(1);
    write_tdef(&mut file, 1, &catalog_columns());
    let def = JetTableDef::decode(&file, 1).expect("decodes");
    assert_eq!(def.columns.len(), 3);
    assert_eq!(def.columns[1].name, "Größe");
    assert!(def.columns[0].is_fixed());
    assert!(!def.columns[1].is_fixed());
}

#[test]
fn jet3_rows_decode_text_mask_and_booleans() {
    let mut file = shell(2);
    write_tdef(&mut file, 1, &catalog_columns());
    let mut fixed = vec![0u8; 4];
    fixed.copy_from_slice(&7i32.to_le_bytes());
    // "helloß" as CP1252 bytes.
    let mut name = b"hello".to_vec();
    name.push(0xDF);
    let row = jet3_row(3, &fixed, &[&name], &[true, true, true]);
    write_data_page(&mut file, 2, 1, std::slice::from_ref(&row));
    let def = JetTableDef::decode(&file, 1).expect("tdef");
    let raw = last_row_bytes(&file, 2, row.len());
    let values = decode_row_jet3(&raw, &def, &file, PAGE).expect("decodes");
    assert_eq!(values[0], StorageValue::Int(7));
    assert_eq!(values[1], StorageValue::Text("hello\u{df}".into()));
    assert_eq!(values[2], StorageValue::Bit(true));

    // NULL name and false boolean via the mask.
    let row = jet3_row(3, &fixed, &[b""], &[true, false, false]);
    write_data_page(&mut file, 2, 1, std::slice::from_ref(&row));
    let raw = last_row_bytes(&file, 2, row.len());
    let values = decode_row_jet3(&raw, &def, &file, PAGE).expect("decodes");
    assert_eq!(values[1], StorageValue::Null);
    assert_eq!(values[2], StorageValue::Bit(false));
}

#[test]
fn jet3_fixture_reads_through_the_engine_boundary() {
    let mut file = shell(3);
    write_tdef(&mut file, 1, &catalog_columns());
    let mut fixed = vec![0u8; 4];
    fixed.copy_from_slice(&1i32.to_le_bytes());
    let row = jet3_row(3, &fixed, &[b"alpha"], &[true, true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("the reader opens Jet3 files");
    let data = reader.read_table(1).expect("reads");
    assert_eq!(data.schema.columns.len(), 3);
    assert_eq!(data.rows.len(), 1);
    assert_eq!(data.rows[0][1], StorageValue::Text("alpha".into()));
}

#[test]
fn jet3_rows_over_255_data_bytes_are_a_precise_unsupported() {
    let mut file = shell(2);
    write_tdef(&mut file, 1, &catalog_columns());
    // A row whose variable data crosses 255 bytes needs the jump table:
    // the eod wraps and the byte pattern stops matching the tail layout.
    let fixed = vec![0u8; 4];
    let big = vec![b'x'; 250];
    let row = jet3_row(3, &fixed, &[&big, b"y"], &[true, true, true]);
    write_data_page(&mut file, 2, 1, std::slice::from_ref(&row));
    let def = JetTableDef::decode(&file, 1).expect("tdef");
    let raw = last_row_bytes(&file, 2, row.len());
    let err = decode_row_jet3(&raw, &def, &file, PAGE).unwrap_err();
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err}");
}

// --- hostile-input paths: every Jet3 decode failure is precise ---

#[test]
fn jet3_tdef_rejects_non_tdef_pages_and_chains() {
    let mut file = shell(2);
    let err = JetTableDef::decode(&file, 1).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
    write_tdef(&mut file, 1, &catalog_columns());
    let base = PAGE; // TDEF page header: next-page dword at offset 4
    file[base + 4..base + 8].copy_from_slice(&7u32.to_le_bytes());
    let err = JetTableDef::decode(&file, 1).unwrap_err();
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err}");
}

#[test]
fn jet3_tdef_rejects_implausible_block_lengths() {
    let mut file = shell(1);
    write_tdef(&mut file, 1, &catalog_columns());
    file[8..12].copy_from_slice(&10u32.to_le_bytes()); // tdef_len < 35
    let err = JetTableDef::decode(&file, 1).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
    file[8..12].copy_from_slice(&999_999u32.to_le_bytes()); // beyond the page
    let err = JetTableDef::decode(&file, 1).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
}

#[test]
fn jet3_data_pages_reject_truncation_and_wrong_owners() {
    let err = datacraft_storage::jet::DataPage::decode_jet3(&[0x01, 0x01, 0, 0]).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
    let err = datacraft_storage::jet::DataPage::decode_jet3(&[0x02; PAGE]).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
    let mut page = [0u8; PAGE];
    page[0] = 0x01;
    page[8..10].copy_from_slice(&9999u16.to_le_bytes()); // count beyond capacity
    let err = datacraft_storage::jet::DataPage::decode_jet3(&page).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
}

#[test]
fn jet3_rows_reject_short_tails() {
    let mut file = shell(2);
    write_tdef(&mut file, 1, &catalog_columns());
    let def = JetTableDef::decode(&file, 1).expect("tdef");
    let err = decode_row_jet3(&[], &def, &file, PAGE).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
    // Truncated tail: num_cols claims a mask that does not fit.
    let err = decode_row_jet3(&[3, 0, 0], &def, &file, PAGE).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
}

#[test]
fn jet3_data_pages_reject_descending_order_violations_and_overflow_slots() {
    // Overflow slots are accepted and carry 4-byte pointers.
    let mut page = [0u8; PAGE];
    page[0] = 0x01;
    page[8..10].copy_from_slice(&1u16.to_le_bytes());
    page[10..12].copy_from_slice(&(0x4000u16 | 100).to_le_bytes());
    let decoded = datacraft_storage::jet::DataPage::decode_jet3(&page).expect("overflow slot");
    assert!(decoded.is_overflow(0));
    assert_eq!(decoded.record(0).0, 100);

    // A record at or above the previous record end is corrupt.
    let mut page = [0u8; PAGE];
    page[0] = 0x01;
    page[8..10].copy_from_slice(&2u16.to_le_bytes());
    page[10..12].copy_from_slice(&200u16.to_le_bytes());
    page[12..14].copy_from_slice(&200u16.to_le_bytes());
    let err = datacraft_storage::jet::DataPage::decode_jet3(&page).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
}
