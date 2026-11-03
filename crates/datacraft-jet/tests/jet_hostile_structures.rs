//! Hostile-structure regressions from the phase 34 review (H46, M28).
//!
//! Every case here reproduced a panic or a silent truncation before the
//! fixes; the suite keeps the precise-error contract pinned.

use datacraft_jet::jet::{Jet4TableReader, JetColumn, JetType};

mod support;

use support::{PAGE, build_row, file_shell};

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

/// The global free map grows when an appended page lands beyond its
/// window (the checked window arithmetic in `append_page`).
#[test]
fn the_free_map_window_grows_for_a_page_beyond_it() {
    use datacraft_jet::jet::Jet4Writer;
    use datacraft_jet::jet::usage_map::UsageMap;
    let mut file = support::synthetic_catalog_db();
    // A one-byte bitmap covers pages 0..7; the file is eight pages long,
    // so the next appended page (8) lies beyond the window.
    let row = UsageMap {
        start_page: 0,
        bitmap: vec![0xFF],
    }
    .encode_row();
    support::write_data_page(&mut file, 1, 1, &[row]);
    file.resize(8 * PAGE, 0);
    let mut writer = Jet4Writer::new(file).expect("writer");
    assert_eq!(writer.allocate_page(2).expect("page 8 allocates"), 8);
}

/// A map page whose second slot spans past the page fails while the map is
/// rewritten, not by indexing out of bounds.
#[test]
fn rewriting_a_map_row_refuses_slots_past_the_page() {
    use datacraft_jet::jet::{DataPage, Jet4Writer};
    let mut file = support::synthetic_catalog_db();
    let decoded = DataPage::decode(&file[PAGE..2 * PAGE]).expect("map page");
    let (start, _) = decoded.record(0);
    let base = PAGE;
    file[base + 12..base + 14].copy_from_slice(&2u16.to_le_bytes());
    file[base + 14..base + 16].copy_from_slice(&(start as u16).to_le_bytes());
    file[base + 16..base + 18].copy_from_slice(&(0x4000u16 | 4095).to_le_bytes());
    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .allocate_page(2)
        .expect_err("map rewrite must fail precisely");
    assert!(err.to_string().contains("beyond the page"), "{err}");
}

/// `encode_row` refuses a column count beyond the u16 row space.
#[test]
fn encode_row_refuses_more_columns_than_a_jet_row_addresses() {
    use datacraft_jet::StorageValue;
    use datacraft_jet::jet::JetTableDef;
    use datacraft_jet::jet::encode::encode_row;

    let columns: Vec<JetColumn> = (0..=u16::MAX)
        .map(|id| JetColumn::new("c", JetType::LongInt, 0x01, id, 0, 4))
        .collect();
    let def = JetTableDef {
        table_type: 0x4E,
        num_rows: 0,
        autonumber: 0,
        columns,
        used_pages: 0,
        free_pages: 0,
        indexes: Vec::new(),
        index_section: Vec::new(),
        index_count: 0,
        real_index_count: 0,
    };
    let values = vec![StorageValue::Null; 65_536];
    let err = encode_row(&values, &def).expect_err("too many columns must fail");
    assert!(err.to_string().contains("65535"), "{err}");
}

/// The memo probe treats a variable index beyond the row's slot count as
/// "no value" and a 1-3 byte slot as an external form.
#[test]
fn the_memo_probe_covers_short_and_out_of_range_slots() {
    use datacraft_jet::StorageValue;
    use datacraft_jet::jet::Jet4Writer;

    // Memo var_index 1 with zero variable slots: probe continues, update
    // succeeds with an empty slot.
    let mut columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    columns[1].var_index = Some(1);
    let mut file = file_shell(3);
    support::write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &[], &[true, true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let mut writer = Jet4Writer::new(file).expect("writer");
    writer
        .update_row(1, 0, &[StorageValue::Int(1), StorageValue::Null])
        .expect("an out-of-range var index is an empty slot");

    // A one-byte memo slot cannot carry the 4-byte flags word: refused.
    let columns = vec![
        JetColumn::new("Id", JetType::LongInt, 0x01, 0, 0, 4),
        JetColumn::new("Note", JetType::Memo, 0x02, 1, 0, 0),
    ];
    let mut file = file_shell(3);
    support::write_tdef(&mut file, 1, 1, 0x4E, &columns);
    let row = support::raw_var_row(2, &1i32.to_le_bytes(), &[0x00], &[true, true]);
    support::write_data_page(&mut file, 2, 1, &[row]);
    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .update_row(1, 0, &[StorageValue::Int(1), StorageValue::Null])
        .expect_err("a short memo slot must be refused");
    assert!(err.to_string().contains("external LVAL"), "{err}");
}

/// A global map page without its row fails with the precise "holds no
/// row" reason instead of indexing the record list.
#[test]
fn an_empty_global_map_page_fails_precisely() {
    use datacraft_jet::jet::Jet4Writer;
    let mut file = support::synthetic_catalog_db();
    support::write_data_page(&mut file, 1, 1, &[]);
    let mut writer = Jet4Writer::new(file).expect("writer");
    let err = writer
        .allocate_page(2)
        .expect_err("an empty map page must fail");
    assert!(err.to_string().contains("holds no row"), "{err}");
}
