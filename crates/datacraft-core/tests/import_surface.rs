//! The import surface honesty (step 91.3, items M111/M112): a database
//! file on the text-import surface answers a format-naming rejection —
//! the healthy file belongs to File → Open, not to the decoder — and the
//! composed error carries the source path exactly once.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-import-surface-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write the probe file");
    path
}

fn core_with_db() -> Core {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )])));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

fn failed_reason(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("the command fails")
}

/// A Jet-family file (the shared `00 01 00 00` page magic plus the Jet4
/// version string, padded past one page start) answers the wrong-surface
/// rejection naming the family and the right surface — never the text
/// decoder's byte error.
#[test]
fn a_jet_file_on_the_import_surface_names_the_format_not_a_decoder_error() {
    let mut bytes = vec![0x00, 0x01, 0x00, 0x00];
    bytes.extend_from_slice(b"Standard Jet DB");
    bytes.resize(128, b' ');
    let path = scratch("probe-jet.mdb", &bytes);
    let mut core = core_with_db();
    let reason = failed_reason(&core.dispatch(Command::PreviewImportFile {
        source: path.clone(),
    }));
    assert!(reason.contains("Jet/ACE"), "the family is named: {reason}");
    assert!(
        reason.contains("File") && reason.to_lowercase().contains("open"),
        "the right surface is named: {reason}"
    );
    assert!(
        !reason.contains("Windows-1252") && !reason.contains("byte 0x"),
        "no decoder byte error leaks: {reason}"
    );
    let _ = std::fs::remove_file(&path);
}

/// A SQLite file answers the same way — the sniff runs before the decode
/// chain for every database family (one probe file per family).
#[test]
fn a_sqlite_file_on_the_import_surface_names_the_format_too() {
    let mut bytes = b"SQLite format 3\0".to_vec();
    bytes.resize(128, b' ');
    let path = scratch("probe-sqlite.db", &bytes);
    let mut core = core_with_db();
    let reason = failed_reason(&core.dispatch(Command::PreviewImportFile {
        source: path.clone(),
    }));
    assert!(reason.contains("SQLite"), "the family is named: {reason}");
    assert!(
        reason.contains("File") && reason.to_lowercase().contains("open"),
        "the right surface is named: {reason}"
    );
    assert!(
        !reason.contains("Windows-1252") && !reason.contains("byte 0x"),
        "no decoder byte error leaks: {reason}"
    );
    let _ = std::fs::remove_file(&path);
}

/// The composed decode error names the path exactly once (M112): the
/// file layer owns the prefix, the command layer forwards the reason
/// untouched. An `0x81` byte is undefined in Windows-1252 and not valid
/// UTF-8, so the decode chain fails with its precise location.
#[test]
fn a_failing_decode_carries_the_path_exactly_once() {
    let path = scratch("broken.csv", &[0x81, 0x00]);
    let mut core = core_with_db();
    let reason = failed_reason(&core.dispatch(Command::PreviewImportFile {
        source: path.clone(),
    }));
    let display = path.display().to_string();
    assert!(reason.contains(&display), "the path is named: {reason}");
    assert_eq!(
        reason.matches(&display).count(),
        1,
        "the path appears exactly once: {reason}"
    );
    assert!(
        reason.contains("byte 0x81 at offset 0"),
        "the precise inner location survives: {reason}"
    );
    let _ = std::fs::remove_file(&path);
}
