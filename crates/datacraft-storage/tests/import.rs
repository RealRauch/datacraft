//! The Import File chain (step 58.1, item M50): one command parses the
//! external CSV source, creates the table and fills it over the ordinary
//! write path — Jet4 and SQLite alike — with the column kinds inferred
//! by the L7 rules. RED state: `Command::ImportFile` answers a precise
//! not-implemented failure yet.

use std::path::{Path, PathBuf};
use std::process::id;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::DatabaseFormat;
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_sqlite::SqlitePort;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-import-{name}-{}", id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The CSV source: a header row plus two records — `sku`/`qty` are
/// strict integers (LongInteger), `label` stays Short Text.
fn sample_csv(dir: &Path) -> PathBuf {
    let path = dir.join("inventory.csv");
    std::fs::write(&path, "sku,label,qty\n1,bolt,50\n2,nut,120\n").expect("write the source csv");
    path
}

fn assert_import_chain(format: DatabaseFormat) {
    let dir = scratch(match format {
        DatabaseFormat::Sqlite => "sqlite",
        _ => "jet4",
    });
    let db_path = match format {
        DatabaseFormat::Sqlite => dir.join("import.sqlite"),
        _ => dir.join("import.mdb"),
    };
    let _ = std::fs::remove_file(&db_path);
    let port: Box<dyn datacraft_core::port::StoragePort> = match format {
        DatabaseFormat::Sqlite => Box::new(SqlitePort::new()),
        _ => Box::new(Jet4Port::new()),
    };
    let mut core = Core::new(port);
    core.dispatch(Command::CreateDatabase {
        path: db_path.clone(),
        format,
    });
    core.dispatch(Command::OpenDatabase {
        path: db_path,
        password: None,
    });

    let source = sample_csv(&dir);
    // The source's bytes before the import — M50: the import never
    // touches the file it reads (58.2 pin).
    let source_before = std::fs::read(&source).expect("the source exists");
    let events = core.dispatch(Command::ImportFile {
        source: source.clone(),
        table: "inventory".into(),
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "the import chain runs over the write path: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::TableCreated { table } if table == "inventory")),
        "the table is created: {events:?}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::RowInserted { .. }))
            .count(),
        2,
        "every source record is inserted: {events:?}"
    );

    // The inferred schema rides the snapshot (L7 rules: strict integers
    // → LongInteger, everything else → Short Text).
    let db = core.inspect().database.expect("the database is open");
    let table = db
        .tables
        .iter()
        .find(|table| table.name == "inventory")
        .expect("the imported table is catalogued");
    assert_eq!(table.fields.len(), 3);
    assert_eq!(table.fields[0].name, "sku");
    assert!(matches!(
        table.fields[0].kind,
        datacraft_core::schema::FieldKind::Number(_)
    ));
    assert!(matches!(
        table.fields[1].kind,
        datacraft_core::schema::FieldKind::Text { .. }
    ));

    // The source file stays byte-identical (58.2): the import reads, it
    // never writes back.
    let source_after = std::fs::read(&source).expect("the source still exists");
    assert_eq!(
        source_before, source_after,
        "the import source is byte-identical after the import"
    );
}

#[test]
fn a_csv_imports_into_an_open_sqlite_database_over_the_write_path() {
    assert_import_chain(DatabaseFormat::Sqlite);
}

#[test]
fn a_csv_imports_into_an_open_jet4_database_over_the_write_path() {
    assert_import_chain(DatabaseFormat::Jet4);
}
