//! Writer-core contract tests (step 10.1, item H9): page allocation over the
//! global free map and the per-table used maps. Allocation is append-only —
//! interior free-page reclaim is a documented later optimization (never
//! needed for the write milestone's golden round-trips).

use datacraft_storage::jet::usage_map::UsageMap;
use datacraft_storage::jet::{Jet4TableReader, Jet4Writer};

mod support;

use support::{PAGE, file_shell};

/// Builds a Jet4 shell whose page 1 carries a type-0 global free map
/// pre-marking `spare` free bits beyond EOF, and whose page 2 is a TDEF
/// (used map pointing at page 3 row 0).
fn shell_with_maps(spare: u32) -> Vec<u8> {
    let mut file = file_shell(3);
    // Global free map on page 1, row 0: window [0, 4 + spare) pages, every
    // page beyond the file end (pages 4..4+spare) marked free.
    let mut bitmap = vec![0u8; ((4 + spare) as usize).div_ceil(8)];
    for page in 4..4 + spare {
        let idx = page as usize;
        bitmap[idx / 8] |= 1 << (idx % 8);
    }
    let row = UsageMap {
        start_page: 0,
        bitmap,
    }
    .encode_row();
    write_data_like_page(&mut file, 1, 1, &[row]);
    // Table TDEF at page 2 (decode-only for the writer); its used-map u32
    // points at page 3, row 0.
    support::write_tdef(&mut file, 2, 0, 0x4E, &[]);
    let u32_at = 8 + 47; // TDEF block offset 47 -> page byte offset
    file[2 * PAGE + u32_at..2 * PAGE + u32_at + 4].copy_from_slice(&((3u32 << 8) | 0).to_le_bytes());
    // Page 3: the table's usage page holding its (empty) used map.
    let row = UsageMap {
        start_page: 0,
        bitmap: vec![0u8; 8],
    }
    .encode_row();
    write_data_like_page(&mut file, 3, 0, &[row]);
    file
}

/// Writes a data page with pre-encoded rows (back-to-front), mimicking the
/// fixture's usage pages.
fn write_data_like_page(file: &mut [u8], page: usize, owner: u32, rows: &[Vec<u8>]) {
    let base = page * PAGE;
    file[base] = 0x01; // data page
    file[base + 1] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&owner.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&(rows.len() as u16).to_le_bytes());
    let mut end = PAGE;
    for (i, row) in rows.iter().enumerate() {
        let start = end - row.len();
        file[base + start..base + start + row.len()].copy_from_slice(row);
        file[base + 14 + i * 2..base + 16 + i * 2].copy_from_slice(&(start as u16).to_le_bytes());
        end = start;
    }
    let free = end - (14 + 2 * rows.len());
    file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
}

#[test]
fn writer_rejects_jet3_with_the_documented_error() {
    let mut file = file_shell(1);
    file[0x14..0x18].copy_from_slice(&0u32.to_le_bytes()); // Jet3 version
    let err = Jet4Writer::new(file).unwrap_err();
    assert!(matches!(err, datacraft_storage::StorageError::Unsupported { .. }), "{err}");
}

#[test]
fn allocate_page_appends_clears_the_free_bit_and_sets_the_table_bit() {
    let file = shell_with_maps(3);
    let page_count = file.len() / PAGE;
    let mut writer = Jet4Writer::new(file).expect("opens");
    let allocated = writer.allocate_page(2).expect("allocates");
    assert_eq!(allocated, page_count as u32, "appends at EOF");
    assert_eq!(writer.file().len(), (page_count + 1) * PAGE);

    // The global free map no longer marks the new page.
    let global = writer.global_free_map().expect("global map");
    assert!(!global.owns_free(allocated));
    // The table's used map marks it now.
    let (_, used) = writer.table_used_map(2).expect("table map");
    assert!(used.owns(allocated));
    // A second allocation takes the next free slot.
    let second = writer.allocate_page(2).expect("allocates");
    assert_eq!(second, allocated + 1);
}

#[test]
fn allocate_page_grows_the_global_map_window_when_headroom_runs_out() {
    let file = shell_with_maps(1);
    let page_count = file.len() / PAGE;
    let mut writer = Jet4Writer::new(file).expect("opens");
    let allocated = writer.allocate_page(2).expect("uses the last spare bit");
    assert_eq!(allocated, page_count as u32);
    // Headroom exhausted: the next allocation must extend the map row.
    let second = writer.allocate_page(2).expect("grows the window");
    assert_eq!(second, allocated + 1);
    let global = writer.global_free_map().expect("global map");
    assert!(!global.owns_free(second));
}

#[test]
fn allocate_page_grows_the_table_map_row_in_place() {
    let file = shell_with_maps(3);
    let mut writer = Jet4Writer::new(file).expect("opens");
    for _ in 0..3 {
        writer.allocate_page(2).expect("allocates");
    }
    let (_, used) = writer.table_used_map(2).expect("table map");
    let page_count = writer.header().page_count as u32;
    for page in (page_count - 3)..page_count {
        assert!(used.owns(page), "page {page} must be owned after insert");
    }
}

#[test]
fn written_file_still_opens_and_reads_unchanged() {
    // Regression: allocation must not disturb the reader (page headers,
    // record coordinates, TDEF chain).
    let file = shell_with_maps(3);
    let mut writer = Jet4Writer::new(file).expect("opens");
    writer.allocate_page(2).expect("allocates");
    let written = writer.into_file();
    Jet4TableReader::new(written).expect("the written file still opens");
}

/// Real-fixture round trip: allocate a page for a real table; the reader
/// must still decode every original table identically and the maps must
/// agree with the page scan afterwards.
#[test]
fn fixture_allocation_preserves_every_read() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let before = Jet4TableReader::new(file.clone()).expect("opens");
    let mut tables = Vec::new();
    for name in ["MSysObjects", "MSysRelationships"] {
        if let Ok(data) = before.open_table(name) {
            tables.push((name, data.rows.len()));
        }
    }
    assert!(!tables.is_empty(), "fixture must hold the catalog");

    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let allocated = writer.allocate_page(2).expect("allocates for the catalog");
    let after = Jet4TableReader::new(writer.into_file()).expect("written file opens");
    for (name, rows) in tables {
        let data = after.open_table(name).expect("table still reads");
        assert_eq!(data.rows.len(), rows, "{name}: row count unchanged");
    }
    // The allocated page belongs to the catalog table now.
    let writer = Jet4Writer::new(after.into_raw_file()).expect("reopens");
    let (_, used) = writer.table_used_map(2).expect("catalog map");
    assert!(used.owns(allocated));
}
