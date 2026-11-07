//! Shared synthetic Jet4 fixture builders for the decoder test suites.
//! Every helper encodes bytes exactly as documented in `docs/FORMAT-JET.md`.

#![allow(dead_code)]

use datacraft_jet::jet::{JetColumn, PAGE_TYPE_DATA, PAGE_TYPE_TDEF};

pub const PAGE: usize = 4096;

/// Builds a valid Jet4 file shell (header page + `pages` zero pages).
pub fn file_shell(pages: usize) -> Vec<u8> {
    let mut file = vec![0u8; PAGE * (pages + 1)];
    file[0] = 0x00;
    file[1] = 0x01;
    file[2] = 0x00;
    file[3] = 0x00;
    file[4..20].copy_from_slice(b"Standard Jet DB\0");
    file[0x14..0x18].copy_from_slice(&0x0000_0001u32.to_le_bytes());
    // Real files store the page-0 header region RC4-encrypted with the fixed
    // key (step 16.2 / M5); the reader decrypts it back to these zeros.
    let region = &mut file[0x18..0x18 + datacraft_jet::jet::crypto::JET4_HEADER_REGION];
    datacraft_jet::jet::crypto::rc4(&datacraft_jet::jet::crypto::HEADER_RC4_KEY, region);
    file
}

/// Writes a Jet4 TDEF block at offset 8 of `page`.
#[allow(clippy::too_many_arguments)]
pub fn write_tdef(
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
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes()); // max_cols
    let var_cols = columns.iter().filter(|c| !c.is_fixed()).count();
    block.extend_from_slice(&(var_cols as u16).to_le_bytes());
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
        let name = ucs2(&col.name);
        block.extend_from_slice(&((col.name.chars().count() * 2) as u16).to_le_bytes()); // UCS-2 byte length
        block.extend_from_slice(&name);
    }
    // Variable-column usage-map entries + terminator (real files carry them).
    for col in columns.iter().filter(|c| !c.is_fixed()) {
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
    }
    block.extend_from_slice(&0xFFFFu16.to_le_bytes());
    let len = block.len() as u32; // tdef_len is self-inclusive
    block[..4].copy_from_slice(&len.to_le_bytes());

    let base = tdef_page * PAGE;
    file[base] = PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    file[base + 8..base + 12].copy_from_slice(&(block.len() as u32).to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
}

/// Byte offset where the TDEF block of `tdef_page` begins (page end minus
/// block length — the real-file placement).
pub fn tdef_block_start(block_len: usize) -> usize {
    PAGE - block_len
}

/// Writes a Jet4 data page with the given raw rows (stored back to front).
pub fn write_data_page(file: &mut [u8], page: usize, tdef_page: u32, rows: &[Vec<u8>]) {
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

/// Builds a Jet4 row: fixed values, variable values (UCS-2LE text), null
/// mask. `null_mask_bits` holds one bool per column: `true` = non-null (or
/// the bit value for boolean columns).
pub fn build_row(
    num_cols: u16,
    fixed: &[u8],
    var_values: &[&str],
    null_mask_bits: &[bool],
) -> Vec<u8> {
    let encoded: Vec<Vec<u8>> = var_values.iter().map(|v| ucs2(v)).collect();
    let refs: Vec<&[u8]> = encoded.iter().map(|v| v.as_slice()).collect();
    raw_var_row_n(num_cols, fixed, &refs, null_mask_bits)
}

/// Raw-variable variant of [`build_row`]: variable sections are opaque bytes
/// (memo headers are binary structures, not text).
pub fn raw_var_row(num_cols: u16, fixed: &[u8], var: &[u8], null_mask_bits: &[bool]) -> Vec<u8> {
    raw_var_row_n(num_cols, fixed, &[var], null_mask_bits)
}

fn raw_var_row_n(num_cols: u16, fixed: &[u8], vars: &[&[u8]], null_mask_bits: &[bool]) -> Vec<u8> {
    let mut row: Vec<u8> = Vec::new();
    row.extend_from_slice(&num_cols.to_le_bytes());
    row.extend_from_slice(fixed);
    for var in vars {
        row.extend_from_slice(var);
    }
    let eod = row.len() as u16;
    row.extend_from_slice(&eod.to_le_bytes());
    // var_table: reverse order — entry 0 is the offset of the LAST var column.
    let mut starts: Vec<u16> = Vec::new();
    let mut pos = 2u16 + fixed.len() as u16;
    for var in vars {
        starts.push(pos);
        pos += var.len() as u16;
    }
    for start in starts.iter().rev() {
        row.extend_from_slice(&start.to_le_bytes());
    }
    row.extend_from_slice(&(vars.len() as u16).to_le_bytes());
    let mask_len = (num_cols as usize).div_ceil(8);
    let mut mask = vec![0u8; mask_len];
    for (i, non_null) in null_mask_bits.iter().enumerate() {
        if *non_null {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    row.extend_from_slice(&mask);
    row
}

/// Encodes a string as UCS-2LE (memo payloads are text in column encoding).
pub fn ucs2(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Writes a Jet4 TDEF that carries one real and one logical index plus a
/// variable-column usage-map entry — exercises the full index-skip path.
pub fn write_tdef_with_indexes(
    file: &mut [u8],
    tdef_page: usize,
    num_rows: u32,
    columns: &[JetColumn],
) {
    let mut block: Vec<u8> = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len, patched below
    block.extend_from_slice(&0u32.to_le_bytes()); // unknown
    block.extend_from_slice(&num_rows.to_le_bytes());
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x01);
    block.extend_from_slice(&[0u8; 3]);
    block.extend_from_slice(&0u32.to_le_bytes()); // complex autonumber
    block.extend_from_slice(&[0u8; 8]);
    block.push(0x4E); // table_type: user table
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes()); // max_cols
    let var_cols = columns.iter().filter(|c| !c.is_fixed()).count() as u16;
    block.extend_from_slice(&var_cols.to_le_bytes());
    block.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    block.extend_from_slice(&1u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&1u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // used_pages
    block.extend_from_slice(&0u32.to_le_bytes()); // free_pages
    // Real-index pre-block: 12 bytes per real index.
    block.extend_from_slice(&[0u8; 12]);
    for col in columns {
        block.push(col.data_type as u8);
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&col.var_index.unwrap_or(0).to_le_bytes());
        block.extend_from_slice(&col.column_id.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.push(col.flags);
        block.push(0);
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&col.fixed_offset.to_le_bytes());
        block.extend_from_slice(&col.len.to_le_bytes());
    }
    for col in columns {
        let name = ucs2(&col.name);
        block.extend_from_slice(&((col.name.chars().count() * 2) as u16).to_le_bytes()); // UCS-2 byte length
        block.extend_from_slice(&name);
    }
    let (section, _, _) =
        datacraft_jet::jet::index::encode_section(&[datacraft_jet::jet::index::JetIndex {
            name: "ByName".to_owned(),
            columns: vec![1],
            unique: false,
            required: false,
            primary: false,
            foreign: false,
        }]);
    block.extend_from_slice(&section);
    block.extend_from_slice(&0xFFFFu16.to_le_bytes()); // terminator
    let len = block.len() as u32; // tdef_len is self-inclusive
    block[..4].copy_from_slice(&len.to_le_bytes());

    let base = tdef_page * PAGE;
    file[base] = PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    file[base + 8..base + 12].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
}

/// Resolves `DATACRAFT_ORACLE_MDB` like the oracle suite: as given, or
/// relative to the workspace root two levels up. `None` skips the gated
/// real-fixture tests.
pub fn oracle_fixture() -> Option<std::path::PathBuf> {
    let raw = std::env::var("DATACRAFT_ORACLE_MDB").ok()?;
    let direct = std::path::Path::new(&raw);
    if direct.exists() {
        return Some(direct.to_path_buf());
    }
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(&raw);
    workspace.exists().then_some(workspace)
}

/// Builds a hermetic Jet4 database with a working (empty) MSysObjects
/// catalog: page 1 global free map, page 2 catalog TDEF, page 3 catalog
/// usage page. Built with the crate's own encoders (dogfooding).
pub fn synthetic_catalog_db() -> Vec<u8> {
    use datacraft_jet::jet::encode::{build_table_def, encode_tdef};
    use datacraft_jet::jet::usage_map::UsageMap;
    use datacraft_jet::jet::{JetType, page_type};

    let columns = vec![
        named(JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4), None),
        named(
            JetColumn::new("ParentId", JetType::LongInt, 0x03, 1, 4, 4),
            None,
        ),
        named(
            JetColumn::new("Name", JetType::Text, 0x02, 2, 0, 60),
            Some(0),
        ),
        named(JetColumn::new("Type", JetType::Int, 0x03, 3, 12, 2), None),
        named(
            JetColumn::new("DateCreate", JetType::DateTime, 0x03, 4, 14, 8),
            None,
        ),
        named(
            JetColumn::new("DateUpdate", JetType::DateTime, 0x03, 5, 22, 8),
            None,
        ),
        named(
            JetColumn::new("Owner", JetType::Binary, 0x32, 6, 0, 510),
            Some(1),
        ),
        named(
            JetColumn::new("Flags", JetType::LongInt, 0x03, 7, 30, 4),
            None,
        ),
    ];
    let used = 3u32 << 8;
    let def = build_table_def(0x53, 0, &columns, used, used | 1);

    let mut file = file_shell(3);
    // Page 1: global free map — pages 0..4 allocated, 4..511 free.
    let mut bitmap = vec![0u8; 64];
    for page in 4..64 * 8 {
        bitmap[page as usize / 8] |= 1 << (page % 8);
    }
    write_data_like(
        &mut file,
        1,
        1,
        &[UsageMap {
            start_page: 0,
            bitmap,
        }
        .encode_row()],
    );
    // Page 2: catalog TDEF (no chain), page 3: its usage page.
    let block = encode_tdef(&def);
    let base = 2 * PAGE;
    file[base] = PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    let len = block.len() as u32;
    file[base + 8..base + 12].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
    let empty = UsageMap {
        start_page: 0,
        bitmap: vec![0u8; 8],
    }
    .encode_row();
    write_data_like(&mut file, 3, 0, &[empty.clone(), empty]);
    debug_assert_eq!(page_type(&file[PAGE..2 * PAGE]), 0x01);
    file
}

/// Overrides a column's variable index (dense among variable columns).
fn named(mut col: JetColumn, var: Option<u16>) -> JetColumn {
    col.var_index = var;
    col
}

/// Writes a data page with pre-encoded rows (back-to-front layout).
fn write_data_like(file: &mut [u8], page: usize, owner: u32, rows: &[Vec<u8>]) {
    let base = page * PAGE;
    file[base] = 0x01;
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

/// The committed Access-touched fixture variant (second writer, T3):
/// `DATACRAFT_TOUCHED_MDB` as given, else the workspace path.
pub fn touched_fixture() -> Option<std::path::PathBuf> {
    if let Ok(raw) = std::env::var("DATACRAFT_TOUCHED_MDB") {
        let direct = std::path::Path::new(&raw);
        if direct.exists() {
            return Some(direct.to_path_buf());
        }
    }
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000-access-touched.mdb");
    workspace.exists().then_some(workspace)
}

/// Physical index leaf roots of a table — the writer maintains them on every
/// row insert, update and delete (step 49.2, item M63).
pub fn index_leaf_pages(writer: &datacraft_jet::jet::Jet4Writer, tdef_page: u32) -> Vec<u32> {
    let definition =
        datacraft_jet::jet::JetTableDef::decode(writer.file(), tdef_page).expect("TDEF decodes");
    datacraft_jet::jet::index_pages::physical_indexes(&definition)
        .expect("index metadata decodes")
        .into_iter()
        .filter(|index| index.root_page != 0)
        .map(|index| index.root_page)
        .collect()
}

/// Pages a table creation touches in `MSysACEs`: the ACL table's definition,
/// its data pages and its index leaves (step 49.2, item M63).
pub fn aces_page_set(
    reader: &datacraft_jet::jet::Jet4TableReader,
    writer: &datacraft_jet::jet::Jet4Writer,
) -> Vec<u32> {
    let page = reader
        .catalog()
        .expect("catalog decodes")
        .into_iter()
        .find(|entry| entry.name == "MSysACEs")
        .and_then(|entry| entry.tdef_page)
        .expect("MSysACEs exists");
    let mut pages = vec![page];
    pages.extend(writer.data_pages_of(page));
    pages.extend(index_leaf_pages(writer, page));
    pages
}

/// The drop's expected page set for a fixture table: the TDEF, its data
/// pages, both usage-map rows' page and the physical index leaves (M66).
/// Returns `(freed pages, index pages)` — the index pages additionally
/// assert their release in the global free map.
pub fn drop_freed_pages(
    writer: &datacraft_jet::jet::Jet4Writer,
    tdef_page: u32,
) -> (Vec<u32>, Vec<u32>) {
    let mut freed = vec![tdef_page];
    freed.extend(writer.data_pages_of(tdef_page));
    freed.push(writer.table_used_map(tdef_page).expect("used map").0.page);
    freed.push(writer.table_free_map(tdef_page).expect("free map").0.page);
    let index_pages = index_leaf_pages(writer, tdef_page);
    freed.extend(index_pages.iter().copied());
    (freed, index_pages)
}

/// Asserts every page is released in the global free map after a drop (M66).
pub fn assert_released_in_global_map(
    writer: &datacraft_jet::jet::Jet4Writer,
    pages: &[u32],
    context: &str,
) {
    let global = writer.global_free_map().expect("global free map");
    for page in pages {
        assert!(
            global.owns_free(*page),
            "{context}: page {page} is released in the global map"
        );
    }
}
