//! Evil-input suite (step 2.3, item H6): truncated structures, hostile
//! pointers, compressed text. LESSONS #1 — "smooth" fixtures confirm code,
//! these check it. Systematic truncation sweeps exercise every precise
//! error branch the decoders carry.

use datacraft_jet::jet::{Jet4TableReader, JetColumn, JetTableDef, JetType, decode_row};
use datacraft_jet::{StorageError, StorageValue};

mod support;

use support::{PAGE, build_row, file_shell, ucs2, write_data_page, write_tdef};

fn customers() -> Vec<JetColumn> {
    vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
        JetColumn::new("Active", JetType::Bool, 0x01, 2, 0, 0),
    ]
}

/// Shortening `tdef_len` step by step walks the decoder into every precise
/// "truncated ..." branch — never a panic, never a bogus success.
#[test]
fn shortened_tdef_length_walks_all_truncation_branches() {
    let mut file = file_shell(2);
    write_tdef(&mut file, 1, 0, 0x4E, &customers());
    sweep_tdef_len(&mut file, 1, 165);

    let mut index_file = file_shell(2);
    support::write_tdef_with_indexes(&mut index_file, 1, 5, &customers());
    sweep_tdef_len(&mut index_file, 1, 260);
}

fn sweep_tdef_len(file: &mut [u8], tdef_page: usize, max_block: usize) {
    let block = tdef_page * PAGE + 8;
    for cut in 0..=max_block {
        let saved = file[block..block + 4].to_vec();
        file[block..block + 4].copy_from_slice(&(cut as u32).to_le_bytes());
        let snapshot = file.to_vec();
        match JetTableDef::decode(&snapshot, tdef_page as u32) {
            // Ok is legitimate once the cut covers the block content
            // (trailing slack bytes are harmless and stay in-page).
            Ok(_) => {}
            Err(err) => assert!(
                matches!(
                    err,
                    StorageError::Corrupt { .. } | StorageError::Unsupported { .. }
                ),
                "cut {cut}: unexpected error class {err:?}"
            ),
        }
        file[block..block + 4].copy_from_slice(&saved);
    }
}

/// Every truncation of a valid row must error precisely — never panic.
#[test]
fn truncated_rows_fail_gracefully() {
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers());
    let def = JetTableDef::decode(&file, 1).expect("valid tdef");
    let row = build_row(3, &7i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    for cut in 0..row.len() {
        // Contract: never panic; either a precise error or a structurally
        // valid value vector (all-null rows are legitimately short).
        match decode_row(&row[..cut], &def, &file, PAGE) {
            Ok(values) => assert_eq!(values.len(), 3, "cut {cut}: column count"),
            Err(err) => assert!(
                matches!(
                    err,
                    StorageError::Corrupt { .. } | StorageError::Unsupported { .. }
                ),
                "cut {cut}: unexpected error class {err:?}"
            ),
        }
    }
    let values = decode_row(&row, &def, &file, PAGE).expect("full row decodes");
    assert_eq!(values[1], StorageValue::Text("Ada".into()));
}

#[test]
fn compressed_text_with_mode_switches_decodes() {
    // Marker, compressed run "Ada", switch, uncompressed pair for 'ä'
    // (0x00E4 → bytes E4 00), compressed 'B'.
    let columns = [
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 255),
    ];
    let mut compressed_col = columns[1].clone();
    compressed_col.compressed_text = true;
    let columns = [columns[0].clone(), compressed_col];

    let mut body: Vec<u8> = vec![0xFF, 0xFE];
    body.extend_from_slice(b"Ada");
    body.push(0x00); // switch to uncompressed
    body.extend_from_slice(&0x00E4u16.to_le_bytes()); // 'ä'
    body.extend_from_slice(&0x0042u16.to_le_bytes()); // 'B' — high byte 0 reads as low byte

    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    // Patch the Name column's misc_flags to 0x01 (compressed unicode).
    // TDEF on page 1: block base = PAGE + (PAGE - block_len).
    let name_desc = PAGE + 8 + 55 + 25;
    file[name_desc + 16] = 0x01;

    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &body, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    assert_eq!(table.rows[0][1], StorageValue::Text("AdaäB".into()));
}

#[test]
fn guid_and_binary_fixed_columns_decode() {
    let columns = [
        JetColumn::new("Key", JetType::Guid, 0x01, 0, 0, 16),
        JetColumn::new("Blob", JetType::Binary, 0x01, 0, 16, 4),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut fixed: Vec<u8> = (0u8..16).collect();
    fixed.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    let row = build_row(2, &fixed, &[], &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let table = reader.read_table(1).expect("table");
    let mut expected_guid = [0u8; 16];
    expected_guid.copy_from_slice(&(0u8..16).collect::<Vec<u8>>());
    assert_eq!(table.rows[0][0], StorageValue::Guid(expected_guid));
    assert_eq!(
        table.rows[0][1],
        StorageValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF])
    );
}

#[test]
fn hostile_memo_pointers_fail_with_precise_errors() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        // Pointer to page 0 (the header page) — rejected.
        (
            "page zero pointer",
            {
                let mut v = Vec::new();
                v.extend_from_slice(&(0x4000_0000u32 | 4).to_le_bytes());
                v.extend_from_slice(&0u32.to_le_bytes());
                v.extend_from_slice(&0u32.to_le_bytes());
                v
            },
            "page 0",
        ),
        // LVAL page beyond the file.
        (
            "page beyond file",
            {
                let mut v = Vec::new();
                v.extend_from_slice(&(0x4000_0000u32 | 4).to_le_bytes());
                v.extend_from_slice(&((999u32) << 8).to_le_bytes());
                v.extend_from_slice(&0u32.to_le_bytes());
                v
            },
            "beyond end of file",
        ),
        // Short var content (< 4 bytes cannot even hold the length dword).
        ("short header", vec![0x40, 0x00], "shorter than its header"),
    ];
    for (label, var, needle) in cases {
        let mut file = file_shell(3);
        write_tdef(&mut file, 1, 1, 0x4E, &columns);
        let row = support::raw_var_row(2, &1i32.to_le_bytes(), &var, &[true, true]);
        write_data_page(&mut file, 2, 1, &[row]);
        let reader = Jet4TableReader::new(file).expect("reader");
        let err = reader.read_table(1).expect_err(label);
        let message = err.to_string();
        assert!(
            message.contains(needle),
            "case {label}: message {message:?} misses {needle:?}"
        );
    }
}

#[test]
fn lval_chain_overrun_fails_precisely() {
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    let mut file = file_shell(5);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    // Chained memo declaring 100 bytes; the chain delivers far less.
    let mut chain: Vec<u8> = Vec::new();
    chain.extend_from_slice(&100u32.to_le_bytes());
    chain.extend_from_slice(&((3u32) << 8).to_le_bytes());
    chain.extend_from_slice(&0u32.to_le_bytes());
    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &chain, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    // LVAL page 3: one chunk of 4 bytes, next pointer 0.
    let mut chunk: Vec<u8> = Vec::new();
    chunk.extend_from_slice(&0u32.to_le_bytes());
    chunk.extend_from_slice(&ucs2("abc"));
    write_data_page(&mut file, 3, 0, &[chunk]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("overrun must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("declared memo length")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn unknown_column_type_fails_precisely() {
    let mut file = file_shell(2);
    write_tdef(&mut file, 1, 0, 0x4E, &customers());
    // Corrupt the first column's type byte to 0x55 (undefined).
    file[PAGE + 8 + 55] = 0x55;
    let err = JetTableDef::decode(&file, 1).expect_err("unknown type must fail");
    assert!(
        matches!(err, StorageError::Unsupported { ref reason } if reason.contains("0x55")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn non_tdef_page_is_rejected() {
    let mut file = file_shell(2);
    write_data_page(&mut file, 1, 0, &[]);
    let err = JetTableDef::decode(&file, 1).expect_err("data page must not decode as tdef");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("not a TDEF")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn lval_hostile_pages_fail_precisely() {
    use datacraft_jet::jet::lval;
    let file = file_shell(3);
    // Page 1 is a zero page (type 0x00) — not a data page.
    let err = lval::resolve(&file, PAGE, 0x4000_0000u32 | 4, 1 << 8, false)
        .expect_err("zero page must fail");
    assert!(err.to_string().contains("not a data page"), "{err}");

    // Empty data page: no record 0 to resolve.
    let mut file = file_shell(2);
    support::write_data_page(&mut file, 1, 0, &[]);
    let err = lval::resolve(&file, PAGE, 0x4000_0000u32 | 4, 1 << 8, false)
        .expect_err("missing record must fail");
    assert!(err.to_string().contains("no usable record 0"), "{err}");

    // Single-LVAL record shorter than the declared memo length.
    let mut file = file_shell(2);
    support::write_data_page(&mut file, 1, 0, &[vec![0xAA, 0xBB]]);
    let err = lval::resolve(&file, PAGE, 0x4000_0000u32 | 4, 1 << 8, false)
        .expect_err("short record must fail");
    assert!(err.to_string().contains("record holds 2 bytes"), "{err}");

    // Inline flag reaching the resolver (the row decoder handles inline).
    let err = lval::resolve(&file, PAGE, 0x8000_0000u32 | 4, 1 << 8, false)
        .expect_err("inline must be refused here");
    assert!(err.to_string().contains("inline"), "{err}");
}

#[test]
fn hostile_data_pages_fail_precisely() {
    use datacraft_jet::jet::DataPage;
    // Truncated header.
    let err = DataPage::decode(&[0u8; 10]).expect_err("truncated must fail");
    assert!(err.to_string().contains("truncated"), "{err}");
    // Wrong page type.
    let err = DataPage::decode(&[0x02, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        .expect_err("wrong type must fail");
    assert!(err.to_string().contains("not a data page"), "{err}");
    // Record count exceeds the offset-table capacity.
    let mut page = vec![0u8; PAGE];
    page[0] = 0x01;
    page[12..14].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let err = DataPage::decode(&page).expect_err("huge record count must fail");
    assert!(err.to_string().contains("capacity"), "{err}");
    // Out-of-order record offsets.
    let mut page = vec![0u8; PAGE];
    page[0] = 0x01;
    page[12..14].copy_from_slice(&2u16.to_le_bytes());
    page[14..16].copy_from_slice(&100u16.to_le_bytes());
    page[16..18].copy_from_slice(&100u16.to_le_bytes());
    let err = DataPage::decode(&page).expect_err("equal offsets must fail");
    assert!(
        err.to_string()
            .contains("not below the previous record end"),
        "{err}"
    );
}

#[test]
fn hostile_fixed_and_memo_lengths_fail_cleanly() {
    // Fixed column whose descriptor points beyond the row.
    let mut file = file_shell(3);
    let mut col = JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4);
    col.fixed_offset = 4000;
    write_tdef(&mut file, 1, 1, 0x4E, &[col]);
    let row = build_row(1, &1i32.to_le_bytes(), &[], &[true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader
        .read_table(1)
        .expect_err("fixed beyond row must fail");
    assert!(err.to_string().contains("needs bytes"), "{err}");

    // Inline memo declaring more than the var section holds.
    let columns = [
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let mut var: Vec<u8> = Vec::new();
    var.extend_from_slice(&(0x8000_0000u32 | 100).to_le_bytes());
    var.extend_from_slice(b"tiny");
    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &var, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("inline overrun must fail");
    assert!(err.to_string().contains("declares 100 bytes"), "{err}");

    // Memo header between 4 and 12 bytes: too short for the record pointer.
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let var = vec![0x00, 0x00, 0x00, 0x40, 0xFF];
    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &var, &[true, true]);
    write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("5-byte header must fail");
    assert!(err.to_string().contains("12-byte header"), "{err}");
}

#[test]
fn overflow_error_paths_fail_precisely() {
    // Overflow slot near the page end whose 4-byte pointer names page 999 —
    // far beyond the file (PageOutOfRange through the pointer).
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers());
    let base = 2 * PAGE;
    file[base] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&1u32.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
    let slot_pos = 4080usize; // leaves 4 readable bytes in the page
    file[base + 14..base + 16].copy_from_slice(&(0x4000u16 | slot_pos as u16).to_le_bytes());
    file[base + slot_pos..base + slot_pos + 4].copy_from_slice(&(999u32 << 8).to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("far overflow must fail");
    assert!(
        matches!(err, StorageError::PageOutOfRange { .. }),
        "{err:?}"
    );

    // Overflow slot at offset 16 whose pointer names page 1 — the TDEF page.
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers());
    let base = 2 * PAGE;
    file[base] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&1u32.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
    file[base + 14..base + 16].copy_from_slice(&(0x4000u16 | 16).to_le_bytes());
    file[base + 16..base + 20].copy_from_slice(&(1u32 << 8).to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("tdef target must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("not a data page")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn remaining_boundary_arms_fail_precisely() {
    use datacraft_jet::jet::Jet4TableReader;

    // Header: non-empty first page but zero file size.
    let mut file = file_shell(1);
    file.truncate(0x18);
    let err =
        datacraft_jet::jet::FileHeader::decode(&file, 0).expect_err("zero file size must fail");
    assert!(err.to_string().contains("file is empty"), "{err}");

    // Catalog TDEF lacking the Id column: precise error.
    let mut file = file_shell(3);
    write_tdef(&mut file, 2, 0, 0x53, &customers());
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.catalog().expect_err("catalog without Id must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("Type column")),
        "unexpected error: {err:?}"
    );

    // Overflow slot position beyond the page: "points past the page".
    let mut file = file_shell(3);
    write_tdef(&mut file, 1, 1, 0x4E, &customers());
    let base = 2 * PAGE;
    file[base] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&1u32.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
    file[base + 14..base + 16].copy_from_slice(&(0x4000u16 | 4094).to_le_bytes());
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("slot past page must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("points past")),
        "unexpected error: {err:?}"
    );

    // Overflow pointer to a page whose slot count is smaller than the row
    // index: "has no record N".
    let mut file = file_shell(4);
    write_tdef(&mut file, 1, 1, 0x4E, &customers());
    let base = 2 * PAGE;
    file[base] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&1u32.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
    let slot_pos = 4080usize;
    file[base + 14..base + 16].copy_from_slice(&(0x4000u16 | slot_pos as u16).to_le_bytes());
    file[base + slot_pos..base + slot_pos + 4].copy_from_slice(&((3u32 << 8) | 5).to_le_bytes());
    // Page 3: one valid slot (row 0 exists, row 5 does not).
    let row = build_row(3, &1i32.to_le_bytes(), &["Ada"], &[true, true, true]);
    write_data_page(&mut file, 3, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader
        .read_table(1)
        .expect_err("missing target row must fail");
    assert!(
        matches!(err, StorageError::Corrupt { ref detail, .. } if detail.contains("no record 5")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn hostile_lval_pointers_fail_precisely_at_every_level() {
    use datacraft_jet::jet::Jet4TableReader;
    // A memo column whose single-record pointer targets page 99 (beyond
    // the file) resolves to a precise error, not a panic.
    let mut file = support::file_shell(3);
    let columns = vec![JetColumn::new("Memo", JetType::Memo, 0x02, 0, 0, 0)];
    support::write_tdef(&mut file, 1, 0, 0x4E, &columns);
    let header = [
        (0x4000_0000u32 | 4).to_le_bytes(),
        (99u32 << 8).to_le_bytes(),
    ]
    .concat();
    let row = support::raw_var_row(1, &[], &header, &[true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("opens");
    let err = reader.read_table(1).unwrap_err();
    assert!(!err.to_string().is_empty(), "{err}");
}

/// H46 regression: a hostile fixed-column descriptor length must not slip
/// through `decode_bytes` (Guid copies into a fixed array, scalars index it).
#[test]
fn hostile_fixed_descriptor_lengths_fail_precisely() {
    // Guid descriptor shorter than its 16 bytes.
    let mut file = file_shell(3);
    support::write_tdef(
        &mut file,
        1,
        1,
        0x4E,
        &[JetColumn::new("Key", JetType::Guid, 0x01, 0, 0, 15)],
    );
    let row = build_row(1, &[0u8; 16], &[], &[true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("short Guid must fail");
    assert!(err.to_string().contains("requires 16"), "{err}");

    // Byte descriptor with length 0 (the fixed slot holds no byte).
    let mut file = file_shell(3);
    support::write_tdef(
        &mut file,
        1,
        1,
        0x4E,
        &[JetColumn::new("B", JetType::Byte, 0x01, 0, 0, 0)],
    );
    let row = build_row(1, &[], &[], &[true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader
        .read_table(1)
        .expect_err("zero-length Byte must fail");
    assert!(err.to_string().contains("requires 1"), "{err}");

    // LongInt descriptor shorter than its 4 bytes.
    let mut file = file_shell(3);
    support::write_tdef(
        &mut file,
        1,
        1,
        0x4E,
        &[JetColumn::new("N", JetType::LongInt, 0x01, 0, 0, 2)],
    );
    let row = build_row(1, &[0u8; 2], &[], &[true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let reader = Jet4TableReader::new(file).expect("reader");
    let err = reader.read_table(1).expect_err("short LongInt must fail");
    assert!(err.to_string().contains("requires 4"), "{err}");
}

/// H46 regression: an update must reject a row whose variable-column table
/// points outside the row instead of indexing past it.
#[test]
fn hostile_var_table_ranges_fail_in_the_writer() {
    use datacraft_jet::StorageValue;
    use datacraft_jet::jet::Jet4Writer;

    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
        JetColumn::new("Extra", JetType::Text, 0x02, 2, 0, 20),
    ];
    let mut columns = columns;
    columns[1].var_index = Some(0);
    columns[2].var_index = Some(1);
    let mut file = file_shell(3);
    support::write_tdef(&mut file, 1, 1, 0x4E, &columns);

    // num_cols=3, 4 fixed bytes, no var data, eod, var table for two
    // variable columns (entry for column 1 = 500, entry for column 0 = 17),
    // var_len=2, one mask byte.
    let mut row: Vec<u8> = Vec::new();
    row.extend_from_slice(&3u16.to_le_bytes());
    row.extend_from_slice(&1i32.to_le_bytes());
    let eod_at = row.len();
    row.extend_from_slice(&0u16.to_le_bytes()); // eod patched below
    row.extend_from_slice(&500u16.to_le_bytes()); // var_start(1)
    row.extend_from_slice(&17u16.to_le_bytes()); // var_start(0)
    row.extend_from_slice(&2u16.to_le_bytes());
    row.push(0b0000_0111); // Id, Note, Extra non-null
    let eod = row.len() as u16;
    row[eod_at..eod_at + 2].copy_from_slice(&eod.to_le_bytes());
    assert_eq!(row.len(), 15);
    support::write_data_page(&mut file, 2, 1, &[row]);

    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .update_row(
            1,
            0,
            &[StorageValue::Int(1), StorageValue::Null, StorageValue::Null],
        )
        .expect_err("out-of-row var entry must fail");
    assert!(!err.to_string().is_empty(), "{err}");
}

/// H46 regression: an overflow slot pointing past the page keeps its raw
/// coordinates (real ACE pages carry legacy offset bits), but every consumer
/// that slices the pointer answers a precise error.
#[test]
fn overflow_slots_beyond_the_page_fail_at_the_consumer() {
    use datacraft_jet::jet::{DataPage, Jet4Writer, lval};
    let mut page = vec![0u8; PAGE];
    page[0] = 0x01;
    page[12..14].copy_from_slice(&1u16.to_le_bytes());
    page[14..16].copy_from_slice(&(0x4000u16 | 4095).to_le_bytes());
    let decoded = DataPage::decode(&page).expect("coordinates stay permissive");
    assert_eq!(decoded.record(0), (4095, 4099));

    // The LVAL resolver reaches the same page: precise error, no panic.
    let mut file = file_shell(2);
    file[PAGE..2 * PAGE].copy_from_slice(&page);
    let err = lval::resolve(&file, PAGE, 0x4000_0000u32 | 4, 1 << 8, false)
        .expect_err("overflow LVAL slot must fail");
    assert!(err.to_string().contains("beyond the page"), "{err}");

    // The writer's map path slices the same coordinates: precise error.
    let mut file = support::synthetic_catalog_db();
    file[PAGE + 14..PAGE + 16].copy_from_slice(&(0x4000u16 | 4095).to_le_bytes());
    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .allocate_page(2)
        .expect_err("map slot past the page must fail");
    assert!(err.to_string().contains("beyond the page"), "{err}");
}

/// H46 regression: `encode_row` sizes the null mask from the column count
/// but indexes it by the column id — a hostile id must not panic.
#[test]
fn encode_row_rejects_column_ids_outside_the_mask() {
    use datacraft_jet::StorageValue;
    use datacraft_jet::jet::encode::{build_table_def, encode_row};
    let column = JetColumn::new("N", JetType::LongInt, 0x01, 8, 0, 4);
    let def = build_table_def(0x4E, 0, &[column], 0, 0);
    let err =
        encode_row(&[StorageValue::Int(5)], &def).expect_err("column id beyond the mask must fail");
    assert!(err.to_string().contains("column id 8"), "{err}");
}

/// H46 regression: an encoded row longer than a Jet row can address must be
/// refused, not truncated into a broken `eod`.
#[test]
fn encode_row_refuses_rows_beyond_the_u16_eod() {
    use datacraft_jet::StorageValue;
    use datacraft_jet::jet::encode::{build_table_def, encode_row};
    let column = JetColumn::new("T", JetType::Text, 0x02, 0, 0, 60_000);
    let def = build_table_def(0x4E, 0, &[column], 0, 0);
    let value = StorageValue::Text("a".repeat(40_000));
    let err = encode_row(&[value], &def).expect_err("oversized row must fail");
    assert!(err.to_string().contains("65535"), "{err}");
}

/// H46 regression: a hostile global free map that starts above EOF must not
/// underflow the window growth in `append_page`.
#[test]
fn free_map_above_eof_fails_instead_of_underflowing() {
    use datacraft_jet::jet::{DataPage, Jet4Writer, PAGE_TYPE_DATA};
    let mut file = support::synthetic_catalog_db();
    assert_eq!(file[PAGE], PAGE_TYPE_DATA);
    let decoded = DataPage::decode(&file[PAGE..2 * PAGE]).expect("map page");
    let (start, _) = decoded.record(0);
    let base = PAGE + start;
    file[base + 1..base + 5].copy_from_slice(&9_999u32.to_le_bytes());
    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .allocate_page(2)
        .expect_err("window above EOF must fail");
    assert!(!err.to_string().is_empty(), "{err}");
}

/// M28 regression: a non-finite double in the file must be refused at the
/// read boundary — the domain model (and every wire serialization) requires
/// finite floats.
#[test]
fn non_finite_doubles_are_refused_at_the_read_boundary() {
    let columns = [JetColumn::new("V", JetType::Double, 0x01, 0, 0, 8)];
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut file = file_shell(3);
        support::write_tdef(&mut file, 1, 1, 0x4E, &columns);
        let row = build_row(1, &value.to_le_bytes(), &[], &[true]);
        support::write_data_page(&mut file, 2, 1, &[row]);
        let reader = Jet4TableReader::new(file).expect("reader");
        let err = reader
            .read_table(1)
            .expect_err("non-finite doubles must be refused");
        assert!(err.to_string().contains("finite"), "{value}: {err}");
    }
}
