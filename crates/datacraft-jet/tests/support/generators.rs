//! Deterministic canonical-database generators (step 61.1, item L35).
//!
//! Canonical recipes over the public deterministic materializer
//! (`materialize_jet4_template`, which pins every timestamp — LESSONS #6):
//! an empty database, typed columns and an index. Generators **complement**
//! the binary fixture pins and the libmdb oracle — `fixtures/manifest` is
//! never touched and no generator replaces a pinned fixture.

use std::path::{Path, PathBuf};

use datacraft_core::schema::{FieldDef, FieldKind, IndexMode, NumericKind, TableDef};

/// The pinned creation timestamp every generated database carries — the
/// create template's own date (OLE days; the engine validates the
/// time-of-day, so the value is carried, never derived from `now`).
pub const CREATED: f64 = f64::from_bits(0x40E6_9F9E_3FE5_8AFA);

fn scratch(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("generator scratch dir");
    let path = dir.join(name);
    let _ = std::fs::remove_file(&path);
    path
}

/// A freshly created, empty Jet4 database (empty user catalog).
pub fn generated_empty(_dir: &Path, _name: &str) -> PathBuf {
    unimplemented!("the empty generator (L35)");
}

/// The canonical typed table: the value families the writer maps natively.
pub fn typed_table() -> TableDef {
    TableDef::new(
        "Typed",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("price", FieldKind::Number(NumericKind::Double)),
            FieldDef::new("sold", FieldKind::DateTime),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

/// The canonical indexed table: one `NoDuplicates` text index.
pub fn indexed_table() -> TableDef {
    let mut code = FieldDef::new("code", FieldKind::Text { max_len: 20 });
    code.properties.indexed = IndexMode::NoDuplicates;
    TableDef::new(
        "Indexed",
        vec![
            code,
            FieldDef::new("label", FieldKind::Text { max_len: 50 }),
        ],
    )
}

/// An empty database plus the canonical typed table (no rows).
pub fn generated_typed(_dir: &Path, _name: &str) -> PathBuf {
    unimplemented!("the typed generator (L35)");
}

/// An empty database plus the canonical indexed table (no rows).
pub fn generated_indexed(_dir: &Path, _name: &str) -> PathBuf {
    unimplemented!("the indexed generator (L35)");
}
