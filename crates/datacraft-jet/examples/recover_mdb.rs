//! Recovery tool for Jet4 databases the real engine refuses (phase 49,
//! step 49.4): it re-encodes a database through the create path — the shape
//! the 49.2/49.3 gates verified — instead of copying pages. `SaveDatabaseAs`
//! is a page copy and inherits the source's page-0 state (BACKLOG M67), so
//! an engine-refused source stays refused; this tool decodes schema, rows
//! and relationships and rebuilds everything through the verified writer.
//!
//! Usage:
//!   cargo run -p datacraft-jet --example recover_mdb -- inspect <source>
//!   cargo run -p datacraft-jet --example recover_mdb -- recover <source> <target>
//!
//! `recover` refuses an existing target (the create path never overwrites)
//! and prints a per-table row-count parity summary; any mismatch exits 1.

use std::path::Path;

use datacraft_core::port::{DatabaseFormat, RowRange, StoragePort};
use datacraft_core::schema::{IndexMode, TableDef};
use datacraft_jet::port_adapter::Jet4Port;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args.first().map(String::as_str) {
        Some("inspect") if args.len() == 2 => inspect(Path::new(&args[1])),
        Some("recover") if args.len() == 3 => recover(Path::new(&args[1]), Path::new(&args[2])),
        _ => Err("usage: recover_mdb (inspect <source> | recover <source> <target>)".to_owned()),
    };
    if let Err(message) = outcome {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn inspect(source: &Path) -> Result<(), String> {
    let mut port = Jet4Port::new();
    port.open(source, None).map_err(|e| e.to_string())?;
    let defs = port.tables();
    for def in &defs {
        let count = row_count(&port, &def.name)?;
        println!("table {} ({count} rows)", def.name);
        for field in &def.fields {
            println!(
                "  {} {:?}{}{}",
                field.name,
                field.kind,
                if field.properties.required {
                    ", required"
                } else {
                    ""
                },
                if field.properties.indexed != IndexMode::None {
                    ", indexed"
                } else {
                    ""
                },
            );
        }
    }
    for relationship in port.relationships().map_err(|e| e.to_string())? {
        println!(
            "relationship {}:{} -> {}:{}",
            relationship.table,
            relationship.column,
            relationship.related_table,
            relationship.related_column
        );
    }
    port.close();
    Ok(())
}

fn recover(source: &Path, target: &Path) -> Result<(), String> {
    // Read everything from the source first: the target port takes over
    // afterwards and the two never mix sessions.
    let mut source_port = Jet4Port::new();
    source_port.open(source, None).map_err(|e| e.to_string())?;
    let defs = source_port.tables();
    let mut tables = Vec::new();
    for def in &defs {
        let rows = source_port
            .read_rows(
                &def.name,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .map_err(|e| e.to_string())?;
        println!("read {} ({} rows)", def.name, rows.len());
        tables.push((def.clone(), rows));
    }
    let relationships = source_port.relationships().map_err(|e| e.to_string())?;
    source_port.close();

    let mut target_port = Jet4Port::new();
    target_port
        .create_database(target, DatabaseFormat::Jet4)
        .map_err(|e| e.to_string())?;
    target_port.open(target, None).map_err(|e| e.to_string())?;
    for (def, rows) in &tables {
        create_table(&mut target_port, def)?;
        // `restore_row` writes each image verbatim (AutoNumber identities
        // survive, k5) and advances the target's seed past the highest id.
        for row in rows {
            target_port
                .restore_row(&def.name, row)
                .map_err(|e| e.to_string())?;
        }
        let written = row_count(&target_port, &def.name)?;
        if written != rows.len() {
            return Err(format!(
                "parity mismatch on {}: {} source rows, {written} recovered",
                def.name,
                rows.len()
            ));
        }
        println!("recovered {} ({written} rows)", def.name);
    }
    for relationship in &relationships {
        target_port
            .create_relationship(relationship)
            .map_err(|e| e.to_string())?;
        println!(
            "recovered relationship {}:{} -> {}:{}",
            relationship.table,
            relationship.column,
            relationship.related_table,
            relationship.related_column
        );
    }
    target_port.close();
    Ok(())
}

fn create_table(port: &mut Jet4Port, def: &TableDef) -> Result<(), String> {
    port.create_table(def).map_err(|e| {
        let kinds = def
            .fields
            .iter()
            .map(|f| format!("{} {:?}", f.name, f.kind))
            .collect::<Vec<_>>()
            .join(", ");
        format!("create table {}: {e} (fields: {kinds})", def.name)
    })
}

fn row_count(port: &Jet4Port, table: &str) -> Result<usize, String> {
    let rows = port
        .read_rows(
            table,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(rows.len())
}
