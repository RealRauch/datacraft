//! Excel workbooks as Import File sources (phase 96, step 96.4; item
//! M127).
//!
//! The core owns the `ImportReader` seam; the composition root registers
//! the Excel driver. This suite exercises the real chain: preview →
//! `ImportFile` → created table + typed rows in an open Jet4 database.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::schema::{FieldKind, FieldValue, NumericKind};
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-excel-import-{}-{n}.mdb",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn core() -> (Core, Scratch) {
    let path = scratch_path();
    let scratch = Scratch(path.clone());
    let mut core = Core::new(Box::new(DetectingPort::new()));
    core.register_import_reader(Box::new(datacraft_xlsx::ExcelImport));
    let events = core.dispatch(Command::CreateDatabase {
        path,
        format: datacraft_core::port::DatabaseFormat::Jet4,
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let events = core.dispatch(Command::OpenDatabase {
        path: scratch.0.clone(),
        password: None,
        source: None,
    });
    assert!(
        matches!(events.first(), Some(Event::DatabaseOpened { .. })),
        "{events:?}"
    );
    (core, scratch)
}

#[test]
fn the_preview_reads_the_workbook_through_the_registered_reader() {
    let (mut core, _scratch) = core();
    let events = core.dispatch(Command::PreviewImportFile {
        source: fixture("excelFixture.xlsx"),
    });
    assert!(
        events.is_empty(),
        "the preview is data, not an event: {events:?}"
    );
    let preview = core.inspect().import_preview.expect("preview");
    assert_eq!(preview.table, "excelFixture");
    assert_eq!(preview.total_rows, 2, "the first sheet has two data rows");
    assert_eq!(
        preview
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Name", "Qty", "Price", "Added", "Active", "Note"]
    );
    assert_eq!(preview.fields[3].kind, FieldKind::DateTime);
    assert_eq!(preview.rows[0][0], "Ada");
    assert_eq!(preview.rows[0][3], "2024-01-15");
    assert_eq!(preview.rows[1][5], "hello");
    assert_eq!(preview.dialect, None, "no delimited-text dialect");
}

#[test]
fn import_file_creates_the_table_with_typed_rows() {
    let (mut core, _scratch) = core();
    let events = core.dispatch(Command::ImportFile {
        source: fixture("excelFixture.xlsx"),
        table: "Imported".into(),
    });
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{events:?}"
    );
    let database = core.inspect().database.expect("open");
    let table = database
        .tables
        .iter()
        .find(|table| table.name == "Imported")
        .expect("the imported table exists");
    assert_eq!(
        table
            .fields
            .iter()
            .map(|field| field.kind)
            .collect::<Vec<_>>(),
        vec![
            FieldKind::Text { max_len: 255 },
            FieldKind::Number(NumericKind::LongInteger),
            FieldKind::Number(NumericKind::Double),
            FieldKind::DateTime,
            FieldKind::Boolean { tri_state: false },
            FieldKind::Text { max_len: 255 },
        ]
    );
    let rows = core
        .read_rows(
            "Imported",
            datacraft_core::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("read imported rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][1], FieldValue::Integer(3));
    assert_eq!(rows[0][2], FieldValue::Float(12.5));
    assert_eq!(rows[0][3], FieldValue::DateTime(45_306.0));
    assert_eq!(rows[0][4], FieldValue::Boolean(true));
    assert_eq!(rows[0][5], FieldValue::Null);
    assert_eq!(rows[1][5], FieldValue::Text("hello".into()));
    assert!(!core.inspect().import_window_open);
    assert!(core.inspect().import_preview.is_none());
}
