//! Fresh Jet4 file builder and template materializer (phase 29, steps
//! 29.1/29.2, item H44).
//!
//! A brand-new four-page database — header page, global free map, an empty
//! `MSysObjects` catalog and its usage page — written from nothing. The
//! page shapes are the ones proven readable by the reader/writer suites
//! (`docs/FORMAT-JET.md` "Usage maps (writer)", "Catalog insert recipe");
//! the header region is RC4-encrypted like every real writer leaves it.
//!
//! Determinism (LESSONS #6): the only wall-clock-shaped input is the
//! explicit `created` OLE date — same inputs produce byte-identical files,
//! which is what the sample generator (step 29.3) pins with `--check`.

use std::path::Path;

use datacraft_core::schema::{FieldValue, TableDef};

use crate::error::StorageError;
use crate::jet::crypto::{HEADER_RC4_KEY, JET4_HEADER_REGION};
use crate::jet::encode::{build_table_def, encode_tdef, pack_map_dword};
use crate::jet::header::{JET_PRODUCT, JET4_VERSION, SIGNATURE};
use crate::jet::tdef::JetColumn;
use crate::jet::usage_map::UsageMap;
use crate::jet::writer::Jet4Writer;
use crate::jet::{JetType, PAGE_TYPE_DATA, PAGE_TYPE_TDEF};
use crate::port_write;

/// Jet4 page size (format-derived).
const PAGE: usize = 4096;
/// Page-0 offset of the code page inside the header region.
const CODE_PAGE_AT: usize = 0x3c - 0x18;
/// Page-0 offset of the creation date inside the header region.
const CREATION_DATE_AT: usize = 0x72 - 0x18;
/// Page-0 offset of the password field inside the header region.
const PASSWORD_AT: usize = 0x42 - 0x18;
/// Default database code page of a fresh database (Windows-1252).
const CODE_PAGE: u16 = 1252;
/// Pages the fresh global free map window covers (64-byte bitmap).
const FREE_MAP_WINDOW: u32 = 512;
/// `MSysObjects` TDEF page (format constant, see `catalog.rs`).
const CATALOG_TDEF: u32 = 2;

/// Writes a brand-new Jet4 database file. `created` is the OLE automation
/// date stored in the header (and used as the password mask); the file
/// must not exist yet.
pub fn create_jet4_file(path: &Path, created: f64) -> Result<(), StorageError> {
    if path.exists() {
        return Err(StorageError::Unsupported {
            reason: format!(
                "{}: file already exists — creating would overwrite it",
                path.display()
            ),
        });
    }
    let mut file = vec![0u8; 4 * PAGE];

    // Page 0: magic, product string, version and the encrypted header
    // region (code page, empty password, creation date).
    file[0..4].copy_from_slice(&SIGNATURE);
    file[4..20].copy_from_slice(JET_PRODUCT);
    file[0x14..0x18].copy_from_slice(&JET4_VERSION.to_le_bytes());
    file[0x18..0x18 + JET4_HEADER_REGION].copy_from_slice(&header_region(created));

    // Page 1: the global free map — pages 0..4 allocated, the window
    // beyond EOF marked free.
    let mut bitmap = vec![0u8; (FREE_MAP_WINDOW / 8) as usize];
    for page in 4..FREE_MAP_WINDOW {
        bitmap[page as usize / 8] |= 1 << (page % 8);
    }
    let global_row = UsageMap {
        start_page: 0,
        bitmap,
    }
    .encode_row();
    write_data_page(&mut file, 1, 1, &[global_row]);

    // Page 2: the empty MSysObjects catalog TDEF, page 3: its usage page
    // holding the used (row 0) and free (row 1) map rows.
    let used = pack_map_dword(0x00, 3, 0);
    let free = pack_map_dword(0x00, 3, 1);
    let def = build_table_def(0x53, 0, &catalog_columns(), used, free);
    write_tdef_page(&mut file, CATALOG_TDEF as usize, &encode_tdef(&def));
    let empty = UsageMap {
        start_page: 0,
        bitmap: vec![0u8; 8],
    }
    .encode_row();
    write_data_page(&mut file, 3, 0, &[empty.clone(), empty]);

    std::fs::write(path, &file).map_err(|e| StorageError::Unsupported {
        reason: format!("{}: {e}", path.display()),
    })
}

/// The decrypted header region for a fresh database: code page 1252, an
/// empty password (the creation-date mask itself) and the creation date —
/// RC4-encrypted in place like every real file carries it.
fn header_region(created: f64) -> Vec<u8> {
    let mask = (created as i64 as u32).to_le_bytes();
    let mut region = vec![0u8; JET4_HEADER_REGION];
    region[CODE_PAGE_AT..CODE_PAGE_AT + 2].copy_from_slice(&CODE_PAGE.to_le_bytes());
    // An empty password decodes when every field byte equals the mask byte
    // at its position (four-byte period).
    for (index, slot) in region[PASSWORD_AT..PASSWORD_AT + 40].iter_mut().enumerate() {
        *slot = mask[index % 4];
    }
    region[CREATION_DATE_AT..CREATION_DATE_AT + 8].copy_from_slice(&created.to_le_bytes());
    crate::jet::crypto::rc4(&HEADER_RC4_KEY, &mut region);
    region
}

/// The `MSysObjects` column shape of the fresh catalog (column-name keyed;
/// every writer maps catalog rows by name). Shape proven by the reader and
/// writer suites on real fixtures.
fn catalog_columns() -> Vec<JetColumn> {
    vec![
        JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
        JetColumn::new("ParentId", JetType::LongInt, 0x03, 1, 0, 4),
        JetColumn::new("Name", JetType::Text, 0x02, 2, 0, 60),
        JetColumn::new("Type", JetType::Int, 0x03, 3, 0, 2),
        JetColumn::new("DateCreate", JetType::DateTime, 0x03, 4, 0, 8),
        JetColumn::new("DateUpdate", JetType::DateTime, 0x03, 5, 0, 8),
        JetColumn::new("Owner", JetType::Binary, 0x32, 6, 0, 510),
        JetColumn::new("Flags", JetType::LongInt, 0x03, 7, 0, 4),
    ]
}

/// Writes one data page with pre-encoded rows (back-to-front layout).
fn write_data_page(file: &mut [u8], page: usize, owner: u32, rows: &[Vec<u8>]) {
    let base = page * PAGE;
    file[base] = PAGE_TYPE_DATA;
    file[base + 1] = 0x01;
    file[base + 4..base + 8].copy_from_slice(&owner.to_le_bytes());
    file[base + 12..base + 14].copy_from_slice(&(rows.len() as u16).to_le_bytes());
    let mut end = PAGE;
    for (slot, row) in rows.iter().enumerate() {
        let start = end - row.len();
        file[base + start..base + start + row.len()].copy_from_slice(row);
        file[base + 14 + slot * 2..base + 16 + slot * 2]
            .copy_from_slice(&(start as u16).to_le_bytes());
        end = start;
    }
    let free = end - (14 + 2 * rows.len());
    file[base + 2..base + 4].copy_from_slice(&(free as u16).to_le_bytes());
}

/// Writes a TDEF page around a pre-encoded block.
fn write_tdef_page(file: &mut [u8], page: usize, block: &[u8]) {
    let base = page * PAGE;
    file[base] = PAGE_TYPE_TDEF;
    file[base + 1] = 0x01;
    let len = block.len() as u32;
    file[base + 8..base + 12].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(block);
}

/// Materializes a template — `(definition, seed rows)` per table, in
/// creation order — into a brand-new Jet4 file. Every timestamp (header
/// creation date, catalog rows) is pinned to `created`, so the same
/// template and date produce byte-identical files (LESSONS #6). A failure
/// at any stage removes the partial file.
pub fn materialize_jet4_template(
    path: &Path,
    created: f64,
    tables: &[(TableDef, Vec<Vec<FieldValue>>)],
) -> Result<(), StorageError> {
    create_jet4_file(path, created)?;
    let outcome = materialize_tables(path, created, tables);
    if outcome.is_err() {
        let _ = std::fs::remove_file(path);
    }
    outcome
}

/// Writes the template's tables into the freshly created file.
fn materialize_tables(
    path: &Path,
    created: f64,
    tables: &[(TableDef, Vec<Vec<FieldValue>>)],
) -> Result<(), StorageError> {
    let mut writer = Jet4Writer::new(std::fs::read(path).map_err(|e| unsupported(path, &e))?)?;
    writer.set_catalog_date(created);
    for (definition, rows) in tables {
        let columns = port_write::table_to_columns(definition).map_err(|error| {
            StorageError::Unsupported {
                reason: error.to_string(),
            }
        })?;
        let page = writer.create_table(&definition.name, &columns)?;
        for row in rows {
            let stored =
                port_write::to_storage_values(&definition.fields, row).map_err(|error| {
                    StorageError::Unsupported {
                        reason: error.to_string(),
                    }
                })?;
            writer.insert_row(page, &stored)?;
        }
    }
    std::fs::write(path, writer.into_file()).map_err(|e| unsupported(path, &e))
}

/// Uniform storage error for filesystem failures on a fresh file.
fn unsupported(path: &Path, error: &std::io::Error) -> StorageError {
    StorageError::Unsupported {
        reason: format!("{}: {error}", path.display()),
    }
}
