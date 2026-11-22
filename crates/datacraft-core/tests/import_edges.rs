//! Import File edge contracts (phase 96, step 96.4; item M127): the
//! registry seam, the capability gates and the precise failures around
//! the shared chain.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::import::{ImportReader, ImportTable};
use datacraft_core::port::{DriverCapabilities, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn db_core(caps: DriverCapabilities) -> Core {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_capabilities(caps);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

fn code(events: &[Event]) -> FailureCode {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { code, .. } => Some(*code),
            _ => None,
        })
        .expect("the command fails")
}

/// A reader that never claims a path.
struct NeverClaims;

impl ImportReader for NeverClaims {
    fn supports(&self, _path: &std::path::Path) -> bool {
        false
    }

    fn read(&self, _path: &std::path::Path) -> Result<ImportTable, String> {
        unreachable!("never claims")
    }
}

/// A reader that claims `claim.csv` and fails with its own reason.
struct FailingReader;

impl ImportReader for FailingReader {
    fn supports(&self, path: &std::path::Path) -> bool {
        path.file_name().is_some_and(|name| name == "claim.csv")
    }

    fn read(&self, _path: &std::path::Path) -> Result<ImportTable, String> {
        Err("the claimed source is unreadable".to_owned())
    }
}

#[test]
fn both_import_commands_require_an_open_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    assert_eq!(
        code(&core.dispatch(Command::PreviewImportFile {
            source: PathBuf::from("a.csv"),
        })),
        FailureCode::NoDatabase
    );
    assert_eq!(
        code(&core.dispatch(Command::ImportFile {
            source: PathBuf::from("a.csv"),
            table: "T".into(),
        })),
        FailureCode::NoDatabase
    );
}

#[test]
fn the_chain_gates_on_both_write_affordances_before_reading() {
    let mut read_only = DriverCapabilities::full();
    read_only.create_table = false;
    let mut core = db_core(read_only);
    assert!(matches!(
        code(&core.dispatch(Command::ImportFile {
            source: PathBuf::from("missing.csv"),
            table: "T".into(),
        })),
        FailureCode::Capability(_)
    ));

    let mut no_rows = DriverCapabilities::full();
    no_rows.write_rows = false;
    let mut core = db_core(no_rows);
    assert!(matches!(
        code(&core.dispatch(Command::ImportFile {
            source: PathBuf::from("missing.csv"),
            table: "T".into(),
        })),
        FailureCode::Capability(_)
    ));
}

#[test]
fn an_empty_target_name_and_an_unreadable_source_fail_precisely() {
    let mut core = db_core(DriverCapabilities::full());
    assert_eq!(
        code(&core.dispatch(Command::ImportFile {
            source: PathBuf::from("missing.csv"),
            table: "  ".into(),
        })),
        FailureCode::InvalidInput
    );
    assert_eq!(
        code(&core.dispatch(Command::ImportFile {
            source: PathBuf::from("/definitely/not/here.csv"),
            table: "T".into(),
        })),
        FailureCode::Io
    );
    assert_eq!(
        code(&core.dispatch(Command::PreviewImportFile {
            source: PathBuf::from("/definitely/not/here.csv"),
        })),
        FailureCode::Io
    );
    assert!(core.inspect().import_preview.is_none());
}

#[test]
fn a_declining_reader_falls_through_and_a_claiming_one_owns_the_error() {
    let mut core = db_core(DriverCapabilities::full());
    core.register_import_reader(Box::new(NeverClaims));
    assert_eq!(
        code(&core.dispatch(Command::PreviewImportFile {
            source: PathBuf::from("/definitely/not/here.csv"),
        })),
        FailureCode::Io,
        "the CSV path still answers"
    );

    core.register_import_reader(Box::new(FailingReader));
    let reason = core
        .dispatch(Command::ImportFile {
            source: PathBuf::from("claim.csv"),
            table: "T".into(),
        })
        .into_iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason),
            _ => None,
        })
        .expect("the claimed source fails");
    assert!(
        reason.contains("could not be read") || reason.contains("unreadable"),
        "{reason}"
    );
}
