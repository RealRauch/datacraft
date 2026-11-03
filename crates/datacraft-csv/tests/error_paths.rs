//! CSV driver error-path contracts (step 36.1, item T7): every refusal of
//! the write path names its table and counts, row indexes answer a precise
//! `OutOfRange`, a batch that fails mid-way leaves the file untouched
//! (atomicity), broken files fail the open with a locatable reason, and a
//! failing rewrite leaves no truncated table behind. Complements the
//! happy-path suite in `driver.rs` (phase 30, M18).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, RowWriteOp, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_csv::CsvPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-csv-err-{}-{n}-{stem}",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.0);
    }
}

fn csv_database() -> Scratch {
    let dir = scratch_path("data");
    std::fs::create_dir_all(&dir).expect("create the directory");
    std::fs::write(
        dir.join("people.csv"),
        "name,city\nada,london\ngrace,berlin\n",
    )
    .expect("write the csv file");
    Scratch(dir)
}

fn opened() -> (Scratch, CsvPort) {
    let scratch = csv_database();
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("open the directory");
    (scratch, port)
}

fn two_values(a: &str, b: &str) -> Vec<FieldValue> {
    vec![
        FieldValue::Text(a.to_owned()),
        FieldValue::Text(b.to_owned()),
    ]
}

/// Every refusal on the write path is a `WriteFailed`/`TableNotFound`
/// naming the table and both counts — the port-level arity contract
/// (LESSONS #4: precise and locatable, never "mysteriously does not work").
#[test]
fn csv_write_refusals_name_the_table_and_counts() {
    let (scratch, mut port) = opened();

    // An unknown table answers TableNotFound, not a write failure.
    let error = port
        .insert_row("missing", &two_values("a", "b"))
        .expect_err("unknown table must fail");
    assert!(
        matches!(&error, PortError::TableNotFound(t) if t == "missing"),
        "{error:?}"
    );

    // A wrong arity names the expected and the found count.
    let error = port
        .insert_row("people", &[FieldValue::Text("one".to_owned())])
        .expect_err("insert arity must fail");
    let detail = error.to_string();
    assert!(detail.contains("people"), "table in the error: {detail}");
    assert!(
        detail.contains("expected 2") && detail.contains("found 1"),
        "{detail}"
    );

    let error = port
        .update_row("people", 0, &[FieldValue::Text("one".to_owned())])
        .expect_err("update arity must fail");
    assert!(
        error.to_string().contains("update"),
        "the update arm names itself: {error}"
    );

    // A table created twice answers a write failure naming the table.
    port.create_table(&TableDef::new(
        "fresh",
        vec![FieldDef::new("a", FieldKind::Text { max_len: 255 })],
    ))
    .expect("create once");
    let error = port
        .create_table(&TableDef::new(
            "fresh",
            vec![FieldDef::new("a", FieldKind::Text { max_len: 255 })],
        ))
        .expect_err("a second create of the same table must fail");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
    assert!(error.to_string().contains("already exists"), "{error}");
    let _ = scratch;
}

/// Row indexes beyond the live rows answer the precise `OutOfRange`
/// shape; read windows follow the shared `window_slice` contract (tail
/// window empty, start beyond the count an error).
#[test]
fn csv_row_indexes_answer_out_of_range_and_windows_clamp() {
    let (scratch, mut port) = opened();

    let error = port
        .update_row("people", 9, &two_values("x", "y"))
        .expect_err("update beyond the rows must fail");
    assert!(
        matches!(
            &error,
            PortError::OutOfRange { table, start, row_count, .. }
                if table == "people" && *start == 9 && *row_count == 2
        ),
        "{error:?}"
    );
    let error = port
        .delete_row("people", 2)
        .expect_err("delete at the count must fail");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");

    let rows = port
        .read_rows("people", RowRange { start: 2, len: 5 })
        .expect("the tail window is valid");
    assert!(rows.is_empty(), "tail window is empty");
    let error = port
        .read_rows("people", RowRange { start: 3, len: 1 })
        .expect_err("a window beyond the count must fail");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");
    let _ = scratch;
}

/// One read-modify-write per batch: a mixed batch lands in one rewrite
/// and survives a reopen; a batch whose later operation is invalid
/// rewrites nothing — the file still holds the pre-batch bytes.
#[test]
fn csv_batches_are_atomic_and_validate_every_op() {
    let (scratch, mut port) = opened();
    let before = std::fs::read(scratch.0.join("people.csv")).expect("read the file");

    // A failing batch: a valid update first, then an invalid insert —
    // nothing may be written.
    let error = port
        .write_row_ops(
            "people",
            &[
                RowWriteOp::Update {
                    row: 0,
                    values: two_values("should", "not-land"),
                },
                RowWriteOp::Insert(vec![FieldValue::Text("bad".to_owned())]),
            ],
        )
        .expect_err("the invalid insert must fail the batch");
    assert!(error.to_string().contains("batch insert"), "{error}");
    let after = std::fs::read(scratch.0.join("people.csv")).expect("read the file");
    assert_eq!(before, after, "a failed batch rewrites nothing");

    // Every invalid batch op shape refuses precisely.
    let error = port
        .write_row_ops(
            "people",
            &[RowWriteOp::Update {
                row: 9,
                values: two_values("x", "y"),
            }],
        )
        .expect_err("batch update beyond the rows must fail");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");
    let error = port
        .write_row_ops(
            "people",
            &[RowWriteOp::Update {
                row: 0,
                values: vec![FieldValue::Text("one".to_owned())],
            }],
        )
        .expect_err("batch update arity must fail");
    assert!(error.to_string().contains("batch update"), "{error}");
    let error = port
        .write_row_ops("people", &[RowWriteOp::Delete { row: 9 }])
        .expect_err("batch delete beyond the rows must fail");
    assert!(matches!(error, PortError::OutOfRange { .. }), "{error:?}");
    let error = port
        .write_row_ops(
            "people",
            &[RowWriteOp::Insert(vec![FieldValue::Text("one".to_owned())])],
        )
        .expect_err("batch insert arity must fail");
    assert!(error.to_string().contains("batch insert"), "{error}");

    // The valid mixed batch lands in one call and survives a reopen.
    port.write_row_ops(
        "people",
        &[
            RowWriteOp::Insert(two_values("alan", "wilmslow")),
            RowWriteOp::Update {
                row: 0,
                values: two_values("ada l", "london"),
            },
            RowWriteOp::Delete { row: 1 },
        ],
    )
    .expect("apply the mixed batch");

    let mut reopened = CsvPort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    let rows = reopened
        .read_rows("people", RowRange { start: 0, len: 10 })
        .expect("read after reopen");
    assert_eq!(rows.len(), 2, "2 rows + insert − delete");
    assert_eq!(rows[0][0], FieldValue::Text("ada l".to_owned()));
    assert_eq!(rows[1][0], FieldValue::Text("alan".to_owned()));
}

/// `drop_table` removes the file and the catalog entry, survives a
/// reopen, and answers `TableNotFound` for strangers.
#[test]
fn csv_drop_table_removes_the_file_and_survives_a_reopen() {
    let (scratch, mut port) = opened();
    port.insert_row("people", &two_values("keep", "me"))
        .expect("seed a row");

    let canonical = port.drop_table("PEOPLE").expect("drop case-insensitively");
    assert_eq!(canonical, "people");
    assert!(!scratch.0.join("people.csv").exists(), "the file is gone");
    assert!(port.tables().is_empty(), "the catalog entry is gone");

    let mut reopened = CsvPort::new();
    reopened.open(&scratch.0, None).expect("reopen");
    assert!(reopened.tables().is_empty(), "the drop survived the reopen");

    let error = reopened
        .drop_table("people")
        .expect_err("dropping a stranger must fail");
    assert!(
        matches!(&error, PortError::TableNotFound(t) if t == "people"),
        "{error:?}"
    );
}

/// Broken files fail the open with a locatable reason: non-UTF-8 bytes,
/// a header-less file, an unnamed header column and a broken quote; a
/// leading BOM is stripped instead.
#[test]
fn csv_open_rejects_broken_files_and_strips_a_bom() {
    let scratch = csv_database();

    std::fs::write(scratch.0.join("broken.csv"), [0xFF, 0xFE, 0x41]).expect("write bytes");
    let error = CsvPort::new()
        .open(&scratch.0, None)
        .expect_err("non-UTF-8 content must fail");
    assert!(error.to_string().contains("not valid UTF-8"), "{error}");
    let _ = std::fs::remove_file(scratch.0.join("broken.csv"));

    std::fs::write(scratch.0.join("empty.csv"), "").expect("write an empty file");
    let error = CsvPort::new()
        .open(&scratch.0, None)
        .expect_err("a header-less file must fail");
    assert!(error.to_string().contains("no header row"), "{error}");
    let _ = std::fs::remove_file(scratch.0.join("empty.csv"));

    std::fs::write(scratch.0.join("unnamed.csv"), ",city\nada,london\n")
        .expect("write the unnamed header");
    let error = CsvPort::new()
        .open(&scratch.0, None)
        .expect_err("an unnamed header column must fail");
    assert!(
        error.to_string().contains("column 0 has no name"),
        "{error}"
    );
    let _ = std::fs::remove_file(scratch.0.join("unnamed.csv"));

    std::fs::write(scratch.0.join("quote.csv"), "a,b\n\"unclosed\n").expect("write the quote");
    let error = CsvPort::new()
        .open(&scratch.0, None)
        .expect_err("a broken quote must fail");
    assert!(error.to_string().contains("quote.csv: row"), "{error}");
    let _ = std::fs::remove_file(scratch.0.join("quote.csv"));

    std::fs::write(scratch.0.join("bom.csv"), "\u{feff}name,city\nada,london\n")
        .expect("write the BOM file");
    let mut port = CsvPort::new();
    port.open(&scratch.0, None).expect("a BOM is stripped");
    assert_eq!(
        port.tables()[0].fields[0].name,
        "name",
        "BOM stripped from the header"
    );
}

/// A failing rewrite answers a write failure and leaves the original
/// file byte-identical — the temp-file dance never truncates a table.
#[test]
fn csv_write_failure_keeps_the_original_file() {
    let (scratch, mut port) = opened();
    let before = std::fs::read(scratch.0.join("people.csv")).expect("read the file");
    // A directory at the temp-file path makes every rewrite fail while
    // the real table file stays intact.
    std::fs::create_dir(scratch.0.join("people.csv.tmp")).expect("block the temp path");

    let error = port
        .insert_row("people", &two_values("alan", "wilmslow"))
        .expect_err("the blocked rewrite must fail");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
    let after = std::fs::read(scratch.0.join("people.csv")).expect("read the file");
    assert_eq!(before, after, "the original file is untouched");

    // Reads still work (the failure was in the write path only).
    let rows = port
        .read_rows("people", RowRange { start: 0, len: 1 })
        .expect("reads are unaffected");
    assert_eq!(rows[0][0], FieldValue::Text("ada".to_owned()));
}

/// Non-text domain values render deterministically through the cell
/// display: `Null` becomes an empty cell, numbers render as text.
#[test]
fn csv_cells_render_domain_values_deterministically() {
    let (scratch, mut port) = opened();
    port.insert_row("people", &[FieldValue::Null, FieldValue::Integer(42)])
        .expect("insert the rendered row");
    let rows = port
        .read_rows("people", RowRange { start: 2, len: 1 })
        .expect("read the rendered row");
    assert_eq!(
        rows[0][0],
        FieldValue::Text(String::new()),
        "Null renders empty"
    );
    assert_eq!(rows[0][1], FieldValue::Text("42".to_owned()));
    let _ = scratch;
}

/// `close` clears the session: the catalog empties and every follow-up
/// answers the "no database is open" refusal.
#[test]
fn csv_close_clears_the_session() {
    let (scratch, mut port) = opened();
    assert!(!port.tables().is_empty());
    port.close();
    assert!(port.tables().is_empty(), "close clears the catalog");
    let error = port
        .read_rows("people", RowRange { start: 0, len: 1 })
        .expect_err("reads after close must fail");
    assert!(
        matches!(&error, PortError::Unsupported { reason } if reason.contains("no database is open")),
        "{error:?}"
    );
    let _ = scratch;
}

/// A port that never opened answers the "no database is open" refusal on
/// the table-creation path.
#[test]
fn csv_create_table_without_open_answers_no_database() {
    let mut port = CsvPort::new();
    let error = port
        .create_table(&TableDef::new(
            "late",
            vec![FieldDef::new("a", FieldKind::Text { max_len: 255 })],
        ))
        .expect_err("creating a table without an open must fail");
    assert!(
        matches!(&error, PortError::Unsupported { reason } if reason.contains("no database is open")),
        "{error:?}"
    );
}

/// The drop refuses with a write failure when the file vanished behind
/// the session — and the catalog entry survives (nothing was dropped).
#[test]
fn csv_drop_of_a_vanished_file_answers_a_write_failure() {
    let (scratch, mut port) = opened();
    std::fs::remove_file(scratch.0.join("people.csv")).expect("remove behind the session");
    let error = port
        .drop_table("people")
        .expect_err("dropping a vanished file must fail");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
    assert_eq!(port.tables().len(), 1, "nothing was dropped");
}

/// Creating the directory database fails precisely when a *file* occupies
/// a parent segment of the path.
#[test]
fn csv_create_database_fails_on_a_file_in_the_path() {
    let scratch = Scratch(scratch_path("blocker"));
    std::fs::write(&scratch.0, "i am a file").expect("write the blocker");
    let mut port = CsvPort::new();
    let error = port
        .create_database(&scratch.0.join("child"), DatabaseFormat::Csv)
        .expect_err("a file in the path must fail the creation");
    assert!(matches!(error, PortError::WriteFailed { .. }), "{error:?}");
}

/// Reads fail precisely when the table file vanishes (or is replaced)
/// behind an open session.
#[test]
fn csv_reads_fail_when_the_table_file_vanishes_mid_session() {
    let (scratch, port) = opened();
    std::fs::remove_file(scratch.0.join("people.csv")).expect("remove the table file");
    std::fs::create_dir(scratch.0.join("people.csv")).expect("replace it with a directory");

    let error = port
        .read_rows("people", RowRange { start: 0, len: 1 })
        .expect_err("reading a vanished file must fail");
    assert!(matches!(error, PortError::OpenFailed { .. }), "{error:?}");
    assert!(
        error.to_string().contains("people.csv"),
        "the reason names the file: {error}"
    );
}
