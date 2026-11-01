//! Usage-map contract tests (step 10.1, item H9, writer core).
//!
//! Facts verified against the real Access-written fixture (see
//! `docs/FORMAT-JET.md` "Usage maps"): `used_pages`/`free_pages` in the TDEF
//! pack a map type byte plus a `(page << 8) | row` record pointer; the map
//! itself lives as a row in a data page: `[0x00][start_page u32][bitmap]`,
//! LSB first, bit set = page allocated to the table. The global free map on
//! page 1 row 0 has the opposite polarity: bit set = page free/absent.

use datacraft_storage::jet::{
    Jet4Writer,
    usage_map::{MapRef, UsageMap},
};
use datacraft_storage::{StorageError, jet};

mod support;

/// Builds a type-0 map row: `[0x00][start u32][bitmap]`.
fn map_row(start: u32, bitmap: &[u8]) -> Vec<u8> {
    let mut row = Vec::with_capacity(5 + bitmap.len());
    row.push(0x00);
    row.extend_from_slice(&start.to_le_bytes());
    row.extend_from_slice(bitmap);
    row
}

#[test]
fn map_ref_packs_page_and_row_into_the_tdef_u32() {
    // Observed on the fixture: MSysObjects used map = 0x00000600 -> page 6,
    // row 0; free map = 0x00000601 -> page 6, row 1.
    let (kind, map) = MapRef::split_tdef_u32(0x0000_0600).expect("type 0 map");
    assert_eq!(kind, 0x00);
    assert_eq!(map, MapRef { page: 6, row: 0 });
    let (_, free) = MapRef::split_tdef_u32(0x0000_0601).expect("type 0 map");
    assert_eq!(free, MapRef { page: 6, row: 1 });
    assert_eq!(MapRef { page: 6, row: 18 }.pack(), 0x0000_0612);
}

#[test]
fn map_ref_rejects_indirection_maps_precisely() {
    // The map type byte is the u32's highest byte (verified: 0x00000612 =
    // type 0, pointer 0x612 = page 6 row 18). Type 1 (indirection through
    // 0x05 pages) is not part of the writer milestone; the fixture uses
    // type 0 exclusively.
    let err = MapRef::split_tdef_u32(0x0100_0600).unwrap_err();
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err}");
}

#[test]
fn usage_map_decodes_rows_and_reports_ownership() {
    // tdef 16 of the fixture: start 0, bitmap byte 2 = 0xb8 -> pages 19, 20,
    // 21, 23 (and byte 12 = 0x80 -> 743 in the full row).
    let mut bitmap = vec![0u8; 96];
    bitmap[2] = 0b1011_1000; // pages 19, 20, 21, 23
    bitmap[92] = 0b1000_0000;
    let map = UsageMap::decode_row(&map_row(0, &bitmap)).expect("decodes");
    assert!(map.owns(19));
    assert!(map.owns(20));
    assert!(map.owns(21));
    assert!(map.owns(23));
    assert!(map.owns(743));
    assert!(!map.owns(18));
    assert!(!map.owns(22));
    assert!(!map.owns(744));
}

#[test]
fn usage_map_row_round_trips_byte_identically() {
    let bitmap = [0b0100_0001u8, 0x00, 0xFF];
    let row = map_row(752, &bitmap);
    let map = UsageMap::decode_row(&row).expect("decodes");
    assert_eq!(map.start_page, 752);
    assert_eq!(map.encode_row(), row);
}

#[test]
fn usage_map_set_and_clear_flip_single_bits() {
    let mut map = UsageMap::decode_row(&map_row(0, &[0u8; 8])).expect("decodes");
    map.set(9).expect("in window");
    assert_eq!(map.bitmap, [0, 0b0000_0010, 0, 0, 0, 0, 0, 0]);
    assert!(map.owns(9));
    map.clear(9).expect("in window");
    assert!(!map.owns(9));
    assert_eq!(map.bitmap, [0u8; 8]);
}

#[test]
fn usage_map_grows_the_window_on_set_beyond_the_end() {
    // Access grows map rows when a table's pages move past the initial
    // window (fixture: tdef 5's row covers pages 752..). The writer grows
    // byte-granular and keeps the polarity.
    let mut map = UsageMap::decode_row(&map_row(752, &[0u8; 1])).expect("decodes");
    assert!(!map.covers(8));
    map.set(752 + 15).expect("grows");
    assert_eq!(map.bitmap.len(), 2);
    assert!(map.owns(767));
    // The growth must not disturb earlier bits.
    map.set(752).expect("in window");
    assert!(map.owns(752));
    assert_eq!(map.encode_row()[..5], map_row(752, &[])[..5]);
}

#[test]
fn usage_map_rejects_unknown_row_types() {
    let err = UsageMap::decode_row(&[0x01, 0, 0, 0, 0, 0xFF]).unwrap_err();
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err}");
    let err = UsageMap::decode_row(&[0x00, 0, 0]).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err}");
}

/// Real-fixture verification: every table's used map marks exactly the data
/// pages the owner scan finds — the two independent allocation views agree.
#[test]
fn fixture_usage_maps_agree_with_the_page_scan() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let writer = jet::Jet4Writer::new(file).expect("writer opens the fixture");

    let tdefs = writer.tdef_pages();
    assert!(tdefs.len() >= 5, "fixture holds many tables: {tdefs:?}");
    for tdef in tdefs {
        let (_, map) = writer.table_used_map(tdef).expect("map decodes");
        let scan: Vec<u32> = writer.data_pages_of(tdef).collect();
        let mapped: Vec<u32> = map.marked_pages().collect();
        assert_eq!(mapped, scan, "tdef {tdef}: map and scan must agree");
    }
}

/// The global free map marks no page below EOF (everything up to the end is
/// allocated) and pre-marks headroom beyond it (observed: 3 spare bits).
#[test]
fn fixture_global_free_map_covers_exactly_the_file() {
    let Some(path) = support::oracle_fixture() else {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    };
    let file = std::fs::read(path).expect("fixture readable");
    let writer = jet::Jet4Writer::new(file).expect("writer opens the fixture");
    let map = writer.global_free_map().expect("global map decodes");
    let page_count = writer.header().page_count as u32;
    let marked: Vec<u32> = map.marked_pages().collect();
    assert!(
        marked.iter().all(|p| *p >= page_count),
        "no interior free pages: {marked:?}"
    );
    assert!(
        !marked.is_empty(),
        "headroom beyond EOF is pre-marked free (observed on the fixture)"
    );
}

// --- writer error arms: map lookups fail precisely on broken wiring ---

#[test]
fn map_lookups_name_broken_references_precisely() {
    let file = {
        let mut file = support::file_shell(4);
        support::write_tdef(&mut file, 2, 0, 0x4E, &[]);
        // used_pages points at page 0 — the header page, not a data page.
        let at = 8 + 47;
        file[2 * support::PAGE + at..2 * support::PAGE + at + 4]
            .copy_from_slice(&0u32.to_le_bytes());
        file
    };
    let writer = Jet4Writer::new(file).expect("opens");
    let err = writer.table_used_map(2).unwrap_err();
    assert!(
        err.to_string().contains("not a data page"),
        "the non-data map page is named: {err}"
    );

    // A map page that lacks the referenced row is a precise error too.
    let file = {
        let mut file = support::file_shell(4);
        support::write_tdef(&mut file, 2, 0, 0x4E, &[]);
        let at = 8 + 47;
        file[2 * support::PAGE + at..2 * support::PAGE + at + 4]
            .copy_from_slice(&((3u32 << 8) | 9).to_le_bytes()); // page 3, row 9
        let empty = datacraft_storage::jet::usage_map::UsageMap {
            start_page: 0,
            bitmap: vec![0u8; 8],
        }
        .encode_row();
        // Usage page: a data page (owner 0) holding the one empty row.
        let base = 3 * support::PAGE;
        file[base] = 0x01;
        file[base + 1] = 0x01;
        let start = support::PAGE - empty.len();
        file[base + start..base + start + empty.len()].copy_from_slice(&empty);
        file[base + 12..base + 14].copy_from_slice(&1u16.to_le_bytes());
        file[base + 14..base + 16].copy_from_slice(&(start as u16).to_le_bytes());
        file
    };
    let writer = Jet4Writer::new(file).expect("opens");
    let err = writer.table_used_map(2).unwrap_err();
    assert!(err.to_string().contains("no row 9"), "{err}");
    let _ = MapRef { page: 0, row: 0 };
}

#[test]
fn growing_a_map_below_its_window_start_is_rejected() {
    let mut map = datacraft_storage::jet::usage_map::UsageMap::decode_row(&map_row(100, &[0u8; 8]))
        .expect("decodes");
    let err = map.set(50).unwrap_err();
    assert!(
        matches!(err, datacraft_storage::StorageError::Corrupt { .. }),
        "{err}"
    );
}
