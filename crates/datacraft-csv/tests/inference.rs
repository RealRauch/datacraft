//! Column type inference contracts (step 36.5, item L7): the schema kind
//! of each column is decided at open, deterministically, from a full
//! scan — LongInteger for all-integer columns, Double for all-float,
//! DateTime for all-ISO-date, Short Text for anything else and for every
//! ambiguous column. Quoting is the user's escape hatch: one quoted cell
//! pins the whole column to Short Text. Cell values still read as text.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{RowRange, StoragePort};
use datacraft_core::schema::{FieldKind, NumericKind};
use datacraft_csv::CsvPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-csv-infer-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes the given raw csv text as `data.csv` and opens it; the schema
/// comes back for kind assertions.
fn opened_with(text: &str) -> (Scratch, CsvPort) {
    let scratch = Scratch(scratch_path("db"));
    std::fs::create_dir_all(&scratch.0).expect("create the directory");
    std::fs::write(scratch.0.join("data.csv"), text).expect("write the csv text");
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    (scratch, port)
}

fn kinds_of(port: &CsvPort) -> Vec<FieldKind> {
    port.tables()[0].fields.iter().map(|f| f.kind).collect()
}

fn long_integer() -> FieldKind {
    FieldKind::Number(NumericKind::LongInteger)
}

fn double() -> FieldKind {
    FieldKind::Number(NumericKind::Double)
}

fn date_time() -> FieldKind {
    FieldKind::DateTime
}

fn short_text() -> FieldKind {
    FieldKind::Text { max_len: 255 }
}

/// Consistent columns infer their type: integers, floats, ISO dates;
/// negative integers and exponents infer as numbers too; anything mixed
/// stays Short Text.
#[test]
fn consistent_columns_infer_their_types_and_mixed_stays_text() {
    let (scratch, port) = opened_with(
        "n,price,day,mixed\n\
         7,1.5,2026-11-03,7\n\
         -3,2.25,2026-01-31,2.5\n\
         0,3e2,1999-12-31,abc\n",
    );
    let kinds = kinds_of(&port);
    assert_eq!(
        kinds,
        vec![long_integer(), double(), date_time(), short_text()]
    );
    let _ = scratch;
}

/// One quoted cell pins the whole column to Short Text — quoting is the
/// user's escape hatch, and a column-level schema never guesses over
/// per-cell quoting. Unquoted neighbours don't rescue it.
#[test]
fn quoted_cells_pin_the_column_to_text() {
    let (scratch, port) = opened_with(
        "id,label\n\
         1,\"42\"\n\
         2,42\n",
    );
    let kinds = kinds_of(&port);
    assert_eq!(
        kinds,
        vec![long_integer(), short_text()],
        "the quoted 42 pins column 2"
    );
    let _ = scratch;
}

/// Empty cells don't vote: a column of integers with gaps still infers
/// LongInteger; a fully empty column stays Short Text.
#[test]
fn empty_cells_do_not_vote() {
    let (scratch, port) = opened_with(
        "gappy,blank\n\
         10,\n\
         ,\n\
         12,\n",
    );
    let kinds = kinds_of(&port);
    assert_eq!(kinds, vec![long_integer(), short_text()]);
    let _ = scratch;
}

/// Strict spellings: whitespace and invalid dates keep the column Short
/// Text; a leading plus is a valid float spelling, so `{+7, 8}` is an
/// all-float column and infers Double (strict integers allow only a
/// leading minus — the documented boundary).
#[test]
fn loose_spellings_follow_the_documented_boundary() {
    let (scratch, port) = opened_with(
        "plus,spaced,bad_date,expo_ok\n\
         +7, 42 ,2026-13-01,1e5\n\
         8,43,2026-01-01,2e5\n",
    );
    let kinds = kinds_of(&port);
    assert_eq!(
        kinds,
        vec![double(), short_text(), short_text(), double()],
        "plus-signed numbers are floats; spaces and invalid dates stay text"
    );
    let _ = scratch;
}

/// RFC-4180 quoting parses before inference: embedded commas and
/// newlines inside quotes stay one cell — and pin the column to text.
#[test]
fn quoted_newlines_and_commas_parse_and_stay_text() {
    let (scratch, port) = opened_with(
        "id,note\n\
         1,\"two, commas\"\n\
         2,\"line one\nline two\"\n",
    );
    let kinds = kinds_of(&port);
    assert_eq!(kinds, vec![long_integer(), short_text()]);
    let rows = port
        .read_rows("data", RowRange { start: 0, len: 10 })
        .expect("read the quoted cells");
    assert_eq!(rows.len(), 2, "the embedded newline stayed inside the cell");
    assert_eq!(rows[1][1], text_value("line one\nline two"));
    let _ = scratch;
}

fn text_value(value: &str) -> datacraft_core::schema::FieldValue {
    datacraft_core::schema::FieldValue::Text(value.to_owned())
}

/// The inference is decided at open and stable across opens — the same
/// file opens with the identical schema.
#[test]
fn inference_is_stable_across_reopens() {
    let (scratch, port) = opened_with("n\n1\n2\n");
    let first = kinds_of(&port);
    drop(port);
    let mut reopened = CsvPort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    assert_eq!(kinds_of(&reopened), first, "stable across opens");
    assert_eq!(first, vec![long_integer()]);
}
