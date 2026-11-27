//! Scale-aware decimal cells in the CSV driver's rewrite (phase 104,
//! step 104.1; item M135): a scale-2 decimal writes `25.50`, not the
//! raw scaled integer; scale 0 stays the integer.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::StoragePort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_csv::CsvPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-csv-decimal-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn decimal_field(scale: u8) -> FieldDef {
    FieldDef::new(
        "rate",
        FieldKind::Number(NumericKind::Decimal {
            precision: 18,
            scale,
        }),
    )
}

/// Inserts one row through the port (the atomic rewrite renders the
/// cells) and answers the file's text.
fn rewritten_cell(scale: u8, value: FieldValue) -> String {
    let scratch = Scratch(scratch_path("db"));
    std::fs::create_dir_all(&scratch.0).expect("create the directory");
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    port.create_table(&TableDef::new(
        "Rates",
        vec![decimal_field(scale), FieldDef::new("name", FieldKind::Memo)],
    ))
    .expect("create the table");
    port.insert_row("Rates", &[value, FieldValue::Text("Ada".to_owned())])
        .expect("insert the row");
    std::fs::read_to_string(scratch.0.join("Rates.csv")).expect("read the rewrite")
}

#[test]
fn a_scale_two_decimal_writes_its_declared_scale() {
    let text = rewritten_cell(2, FieldValue::Decimal(2550));
    assert!(
        text.contains("25.50"),
        "the rewrite keeps the declared scale:\n{text}"
    );
    assert!(
        !text.contains("2550"),
        "the raw scaled integer must not appear:\n{text}"
    );
}

#[test]
fn a_scale_zero_decimal_stays_the_integer() {
    let text = rewritten_cell(0, FieldValue::Decimal(2550));
    assert!(
        text.contains("2550"),
        "scale 0 renders byte-identical:\n{text}"
    );
}

#[test]
fn a_read_back_row_rerenders_through_the_same_rule() {
    // The read path serves raw values; the update rewrite renders them —
    // pin that an update keeps the declared scale too.
    let scratch = Scratch(scratch_path("db"));
    std::fs::create_dir_all(&scratch.0).expect("create the directory");
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    port.create_table(&TableDef::new(
        "Rates",
        vec![decimal_field(2), FieldDef::new("name", FieldKind::Memo)],
    ))
    .expect("create the table");
    port.insert_row(
        "Rates",
        &[FieldValue::Decimal(100), FieldValue::Text("Ada".to_owned())],
    )
    .expect("insert the row");
    port.update_row(
        "Rates",
        0,
        &[
            FieldValue::Decimal(2550),
            FieldValue::Text("Grace".to_owned()),
        ],
    )
    .expect("update the row");
    let text = std::fs::read_to_string(scratch.0.join("Rates.csv")).expect("read the rewrite");
    assert!(
        text.contains("25.50"),
        "the update rewrite keeps the declared scale:\n{text}"
    );
}
