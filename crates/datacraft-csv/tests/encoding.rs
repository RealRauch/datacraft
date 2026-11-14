//! The CSV driver reads non-UTF-8 and non-comma tables through the one
//! shared core decode path (step 80.2, item H59): a cp1252 semicolon
//! file, a UTF-16LE BOM file and a tab-separated file open with the
//! right field count and text; invalid content still fails with a
//! locatable reason.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_csv::CsvPort;

fn scratch_dir(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-csv-enc-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open_with(stem: &str, file: &str, bytes: &[u8]) -> (Scratch, CsvPort) {
    let dir = scratch_dir(stem);
    std::fs::create_dir_all(&dir).expect("create the directory");
    std::fs::write(dir.join(file), bytes).expect("write the table file");
    let mut port = CsvPort::new();
    port.open(&dir, None).expect("open the directory");
    (Scratch(dir), port)
}

fn text_of(values: &[Vec<FieldValue>]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| match value {
                    FieldValue::Text(text) => text.clone(),
                    other => panic!("a CSV cell stays text, got {other:?}"),
                })
                .collect()
        })
        .collect()
}

#[test]
fn a_windows1252_semicolon_table_opens_with_its_text() {
    let (scratch, port) = open_with("cp1252", "umlaut.csv", b"a;b\ng\xe4r;tor\n");
    let tables = port.tables();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].fields.len(), 2);
    let rows = port
        .read_rows("umlaut", RowRange { start: 0, len: 10 })
        .expect("read");
    assert_eq!(
        text_of(&rows),
        vec![vec!["gär".to_owned(), "tor".to_owned()]]
    );
    drop(scratch);
}

#[test]
fn a_utf16le_bom_table_opens_with_its_text() {
    let content: Vec<u8> = "\u{feff}a,b\nschön,hier\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let (scratch, port) = open_with("utf16le", "wide.csv", &content);
    let tables = port.tables();
    assert_eq!(tables[0].fields.len(), 2);
    let rows = port
        .read_rows("wide", RowRange { start: 0, len: 10 })
        .expect("read");
    assert_eq!(
        text_of(&rows),
        vec![vec!["schön".to_owned(), "hier".to_owned()]]
    );
    drop(scratch);
}

#[test]
fn a_tab_separated_table_opens_with_two_columns() {
    let (scratch, port) = open_with("tsv", "tabbed.csv", b"a\tb\n1\tx\n");
    let tables = port.tables();
    assert_eq!(tables[0].fields.len(), 2);
    let rows = port
        .read_rows("tabbed", RowRange { start: 0, len: 10 })
        .expect("read");
    assert_eq!(text_of(&rows), vec![vec!["1".to_owned(), "x".to_owned()]]);
    drop(scratch);
}

#[test]
fn invalid_content_still_fails_with_a_locatable_reason() {
    // 0x81 is undefined in Windows-1252 and not valid UTF-8 either; the
    // catalog is read at open, so the open itself refuses.
    let dir = scratch_dir("bad");
    std::fs::create_dir_all(&dir).expect("create the directory");
    std::fs::write(dir.join("broken.csv"), b"a,b\nx,\x81\n").expect("write the table file");
    let mut port = CsvPort::new();
    let error = port
        .open(&dir, None)
        .expect_err("the undefined byte must fail the open");
    assert!(
        matches!(error, PortError::OpenFailed { .. }),
        "the failure is an open failure: {error:?}"
    );
    assert!(
        error.to_string().contains("broken.csv"),
        "the reason names the file: {error}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
