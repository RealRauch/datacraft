//! Deterministic sample generation (phase 29, step 29.3, item H44).
//!
//! The shipped `samples/` set is the output of this module and nothing
//! else: same templates, same pinned [`CREATED`] date, byte-identical
//! files. [`check`] regenerates into a temp directory and byte-compares —
//! exactly what the `make_samples --check` flag runs and what the
//! committed-samples test enforces on every CI run. The Bike Shop demo
//! additionally ships its SQLite twin and a `.datacraft` demo sidecar
//! (form + query, ADR-1).

use std::path::Path;

use datacraft_core::port::{DatabaseFormat, PortError, StoragePort};
use datacraft_core::schema::{FieldValue, SCHEMA_VERSION, SchemaDocument, TableDef};
use datacraft_core::templates;

use datacraft_jet::jet::create::materialize_jet4_template;
use datacraft_sqlite::SqlitePort;

/// The pinned creation date of every shipped sample (OLE automation days —
/// a fixed point in 2025, never the wall clock).
pub const CREATED: f64 = 46_000.0;

/// File names of the shipped sample set, in generation order.
pub const SAMPLE_FILES: [&str; 6] = [
    "blank.mdb",
    "contacts.mdb",
    "inventory.mdb",
    "bike_shop.mdb",
    "bike_shop.sqlite",
    "bike_shop.datacraft",
];

/// Generates the complete sample set into `dir` (created on demand).
pub fn generate_all(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for template in templates::templates() {
        let tables: Vec<(TableDef, Vec<Vec<FieldValue>>)> = template
            .tables
            .iter()
            .map(|table| (table.definition.clone(), table.rows.clone()))
            .collect();
        let path = dir.join(format!("{}.mdb", template.name));
        materialize_jet4_template(&path, CREATED, &tables)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if template.name == "bike_shop" {
            materialize_sqlite_twin(&dir.join("bike_shop.sqlite"), &tables)?;
            write_demo_sidecar(&dir.join("bike_shop.datacraft"), template.name)?;
        }
    }
    Ok(())
}

/// Regenerates the sample set and byte-compares it against `dir`. Every
/// difference names the file and the remedy.
pub fn check(dir: &Path) -> Result<(), String> {
    // Concurrent checks (tests, parallel CI jobs) never share a scratch
    // directory: the counter makes every invocation unique.
    static CHECK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let invocation = CHECK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let scratch = std::env::temp_dir().join(format!(
        "datacraft-samples-check-{}-{invocation}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let outcome = check_in(dir, &scratch);
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

fn check_in(dir: &Path, scratch: &Path) -> Result<(), String> {
    generate_all(scratch)?;
    let mut differing = Vec::new();
    for name in SAMPLE_FILES {
        let generated = std::fs::read(scratch.join(name))
            .map_err(|e| format!("{name}: regeneration failed: {e}"))?;
        let shipped = std::fs::read(dir.join(name)).map_err(|_| {
            format!("{name}: missing from {} — run `cargo run -p datacraft-storage --example make_samples`", dir.display())
        })?;
        if generated != shipped {
            differing.push(name);
        }
    }
    if differing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "regenerated samples differ from {}: {:?} — regenerate with \
         `cargo run -p datacraft-storage --example make_samples` and re-commit",
        dir.display(),
        differing
    ))
}

/// Writes the Bike Shop SQLite twin through the SQLite port.
fn materialize_sqlite_twin(
    path: &Path,
    tables: &[(TableDef, Vec<Vec<FieldValue>>)],
) -> Result<(), String> {
    let mut port = SqlitePort::new();
    if let Err(e) = port.create_database(path, DatabaseFormat::Sqlite) {
        return Err(e.to_string());
    }
    if let Err(e) = materialize_through(&mut port, path, tables) {
        let _ = std::fs::remove_file(path);
        return Err(e.to_string());
    }
    Ok(())
}

fn materialize_through(
    port: &mut SqlitePort,
    path: &Path,
    tables: &[(TableDef, Vec<Vec<FieldValue>>)],
) -> Result<(), PortError> {
    port.open(path, None)?;
    for (definition, rows) in tables {
        port.create_table(definition)?;
        for row in rows {
            port.insert_row(&definition.name, row)?;
        }
    }
    port.close();
    Ok(())
}

/// Writes the demo sidecar: the Bike Finder form and the Expensive bikes
/// query, bound to the demo tables (deterministic JSON bytes).
fn write_demo_sidecar(path: &Path, database_name: &str) -> Result<(), String> {
    let mut form = datacraft_core::form::FormDef::new("Bike Finder");
    form.source = "Bikes".to_owned();
    form.controls = vec![
        datacraft_core::form::ControlDef {
            name: "TitleLabel".to_owned(),
            kind: datacraft_core::form::ControlKind::Label {
                text: "Bike Finder".to_owned(),
            },
            rect: datacraft_core::canvas::Rect {
                x: 16.0,
                y: 16.0,
                width: 120.0,
                height: 24.0,
            },
        },
        datacraft_core::form::ControlDef {
            name: "ModelBox".to_owned(),
            kind: datacraft_core::form::ControlKind::TextBox {
                bound_field: Some("Model".to_owned()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 16.0,
                y: 48.0,
                width: 200.0,
                height: 24.0,
            },
        },
        datacraft_core::form::ControlDef {
            name: "InStockBox".to_owned(),
            kind: datacraft_core::form::ControlKind::CheckBox {
                bound_field: Some("InStock".to_owned()),
            },
            rect: datacraft_core::canvas::Rect {
                x: 16.0,
                y: 80.0,
                width: 24.0,
                height: 24.0,
            },
        },
    ];
    let query = datacraft_core::qbe::QueryDef {
        name: "Expensive bikes".to_owned(),
        table: "Bikes".to_owned(),
        fields: vec![
            datacraft_core::qbe::QbeField {
                name: "Model".to_owned(),
                shown: true,
                sort: datacraft_core::qbe::SortOrder::Ascending,
                criteria: Vec::new(),
            },
            datacraft_core::qbe::QbeField {
                name: "Price".to_owned(),
                shown: true,
                sort: datacraft_core::qbe::SortOrder::None,
                criteria: vec![">300000000".to_owned()],
            },
        ],
    };
    let document = SchemaDocument {
        schema_version: SCHEMA_VERSION,
        database_name: database_name.to_owned(),
        tables: Vec::new(),
        queries: vec![query],
        forms: vec![form],
        reports: Vec::new(),
    };
    std::fs::write(path, document.to_json()).map_err(|e| format!("{}: {e}", path.display()))
}
