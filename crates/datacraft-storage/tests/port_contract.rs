//! The port-level read/write contract, exercised by the real write-capable
//! drivers (step 31.1, item M21): whatever holds for one shipped driver must
//! hold for every other — the successor of the Phase-2 `engine_boundary`
//! suite, which asserted the same rules against the retired in-memory
//! reference backend.
//!
//! The assertions here pin the window semantics of `read_rows` (shared
//! `window_slice`), the unknown-table naming and the arity rejection on
//! **both** Jet4 and SQLite. Sequential batch semantics and update/delete
//! out-of-range errors are pinned per driver (`jet_row_writes`,
//! `sqlite_row_writes`); the error-display precision lives beside its type
//! (`src/error.rs`, `core::port::PortError` arms).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, StoragePort};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_sqlite::SqlitePort;

/// Unique scratch path per test process and call.
fn scratch_path(stem: &str, extension: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-port-contract-{}-{n}-{stem}.{extension}",
        std::process::id()
    ))
}

/// Removes its file when the test ends, whatever the outcome.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The three-column reference table shared by every driver below.
fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Name", FieldKind::Text { max_len: 60 }),
            FieldDef::new("Active", FieldKind::Boolean { tri_state: true }),
        ],
    )
}

/// The reference rows: the fixed `Id` and the variable `Name` columns each
/// carry a NULL. The tri-state Boolean NULL is deliberately absent — the Jet
/// writer cannot encode it yet (M24: a Bool column's value *is* its null-mask
/// bit, so a NULL answers a precise `Unsupported`); SQLite accepts it, and a
/// shared contract must not assert what one shipped driver cannot store.
fn customers_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Integer(1),
            FieldValue::Text("Ada".into()),
            FieldValue::Boolean(true),
        ],
        vec![
            FieldValue::Null,
            FieldValue::Null,
            FieldValue::Boolean(false),
        ],
        vec![
            FieldValue::Integer(3),
            FieldValue::Text("Grace".into()),
            FieldValue::Boolean(false),
        ],
    ]
}

/// One seeded driver session (fresh database, `Customers` with three rows).
struct Driver {
    name: &'static str,
    port: Box<dyn StoragePort>,
    _file: Scratch,
}

impl Driver {
    /// Jet4: create → open → seed through the real port.
    fn jet() -> Self {
        let _file = Scratch(scratch_path("jet", "mdb"));
        let mut port = Jet4Port::new();
        port.create_database(&_file.0, DatabaseFormat::Jet4)
            .expect("create the Jet4 scratch file");
        port.open(&_file.0, None)
            .expect("open the Jet4 scratch file");
        Self::seed("jet", Box::new(port), _file)
    }

    /// SQLite: create → open → seed through the real port.
    fn sqlite() -> Self {
        let _file = Scratch(scratch_path("sqlite", "sqlite"));
        let mut port = SqlitePort::new();
        port.create_database(&_file.0, DatabaseFormat::Sqlite)
            .expect("create the SQLite scratch file");
        port.open(&_file.0, None)
            .expect("open the SQLite scratch file");
        Self::seed("sqlite", Box::new(port), _file)
    }

    fn seed(name: &'static str, mut port: Box<dyn StoragePort>, file: Scratch) -> Self {
        port.create_table(&customers())
            .expect("create the reference table");
        for row in customers_rows() {
            port.insert_row("Customers", &row)
                .expect("insert a reference row");
        }
        Self {
            name,
            port,
            _file: file,
        }
    }

    fn read(&self, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        self.port.read_rows("Customers", range)
    }
}

/// Both shipped write-capable drivers expose the identical read contract.
fn contract_drivers() -> Vec<Driver> {
    vec![Driver::jet(), Driver::sqlite()]
}

#[test]
fn both_drivers_read_identical_windows_with_nulls() {
    for driver in contract_drivers() {
        let rows = driver
            .read(RowRange { start: 1, len: 2 })
            .unwrap_or_else(|e| panic!("{}: read window: {e}", driver.name));
        assert_eq!(rows.len(), 2, "{}: window length", driver.name);
        assert_eq!(
            rows[0][1],
            FieldValue::Null,
            "{}: null passes through",
            driver.name
        );
        assert_eq!(
            rows[1][1],
            FieldValue::Text("Grace".into()),
            "{}: text passes through",
            driver.name
        );
        let all = driver
            .read(RowRange { start: 0, len: 10 })
            .unwrap_or_else(|e| panic!("{}: full read: {e}", driver.name));
        assert_eq!(
            all.len(),
            3,
            "{}: oversized window clamps to the count",
            driver.name
        );
    }
}

#[test]
fn a_window_starting_at_row_count_is_empty_on_both_drivers() {
    for driver in contract_drivers() {
        let rows = driver
            .read(RowRange { start: 3, len: 5 })
            .unwrap_or_else(|e| panic!("{}: tail window: {e}", driver.name));
        assert!(rows.is_empty(), "{}: tail window is empty", driver.name);
    }
}

#[test]
fn a_window_beyond_row_count_is_a_precise_error_on_both_drivers() {
    for driver in contract_drivers() {
        let error = driver
            .read(RowRange { start: 5, len: 2 })
            .expect_err(&format!(
                "{}: a window beyond the count must fail",
                driver.name
            ));
        assert!(
            matches!(
                &error,
                PortError::OutOfRange { table, start, len, row_count }
                    if table == "Customers" && *start == 5 && *len == 2 && *row_count == 3
            ),
            "{}: unexpected error: {error:?}",
            driver.name
        );
    }
}

#[test]
fn reads_from_an_unknown_table_name_the_table_on_both_drivers() {
    for driver in contract_drivers() {
        let error = driver
            .port
            .read_rows("Missing", RowRange { start: 0, len: 1 })
            .expect_err("unknown tables must fail");
        assert!(
            matches!(&error, PortError::TableNotFound(t) if t == "Missing"),
            "{}: unexpected error: {error:?}",
            driver.name
        );
        assert!(
            error.to_string().contains("Missing"),
            "{}: the display names the table: {}",
            driver.name,
            error
        );
    }
}

#[test]
fn wrong_arity_inserts_are_rejected_on_both_drivers() {
    for mut driver in contract_drivers() {
        let error = driver
            .port
            .insert_row("Customers", &[FieldValue::Integer(9)])
            .expect_err("wrong arity must fail");
        let detail = error.to_string();
        assert!(
            detail.contains("1") && detail.contains("3"),
            "{}: the error names both counts: {detail}",
            driver.name
        );
    }
}
