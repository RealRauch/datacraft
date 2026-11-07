//! Detecting-registry edge contracts (step 36.2, item T7) plus the L9
//! format-hint contract: the composite's Debug projection, the fallback
//! path (success *and* combined rejection), the `create_database` routing
//! for foreign formats, and the "no database is open" refusals of the
//! inventory calls. The fallback is a minimal in-test driver on purpose:
//! it proves the composite is driver-agnostic — the real CSV composition
//! is pinned by the ui crate's `server_e2e_csv`.

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, SchemaChange, StoragePort, Support,
};
use datacraft_core::schema::{FieldValue, TableDef};
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-detecting-edges-{}-{n}-{stem}",
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

/// A fallback driver that declines every open unless the path carries the
/// marker suffix — the composite must not care which driver answers.
struct PickyFallback;

impl StoragePort for PickyFallback {
    fn driver_id(&self) -> DriverId {
        DriverId::new("picky")
    }
    fn source_format(&self) -> Option<DatabaseFormat> {
        Some(DatabaseFormat::Csv)
    }
    fn capabilities(&self) -> DriverCapabilities {
        let mut caps = DriverCapabilities::none();
        caps.read_tables = true;
        caps.create_database = true;
        caps
    }
    fn open(&mut self, path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        if path.extension().is_some_and(|ext| ext == "picky") {
            return Ok(OpenedDatabase {
                name: path
                    .file_stem()
                    .map_or_else(|| "picky".to_owned(), |s| s.to_string_lossy().into_owned()),
                format: DatabaseFormat::Csv,
                lock_file: None,
            });
        }
        Err(PortError::OpenFailed {
            detail: format!("{}: not a picky source", path.display()),
        })
    }
    fn close(&mut self) {}
    fn tables(&self) -> Vec<TableDef> {
        Vec::new()
    }
    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "picky".to_owned(),
        })
    }
    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(PortError::Unsupported {
            reason: "picky".to_owned(),
        })
    }
    fn read_rows(&self, _table: &str, _range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        Ok(Vec::new())
    }
    fn insert_row(&mut self, _table: &str, _values: &[FieldValue]) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "picky".to_owned(),
        })
    }
    fn update_row(
        &mut self,
        _table: &str,
        _row: usize,
        _values: &[FieldValue],
    ) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "picky".to_owned(),
        })
    }
    fn delete_row(&mut self, _table: &str, _row: usize) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            reason: "picky".to_owned(),
        })
    }
    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        Ok(Vec::new())
    }
    fn create_database(&mut self, path: &Path, _format: DatabaseFormat) -> Result<(), PortError> {
        std::fs::create_dir_all(path).map_err(|e| PortError::WriteFailed {
            detail: format!("{}: {e}", path.display()),
        })
    }
}

fn picky_factory() -> Box<dyn StoragePort> {
    Box::new(PickyFallback)
}

/// The Debug projection names the composite itself while nothing is open
/// and the open session's driver afterwards; the fallback shows as
/// present.
#[test]
fn the_composite_debug_names_the_session_and_fallback() {
    let plain = DetectingPort::new();
    let debug = format!("{plain:?}");
    assert!(debug.contains("DetectingPort"), "{debug}");
    assert!(debug.contains("fallback: false"), "{debug}");

    let with_fallback = DetectingPort::new().with_fallback(picky_factory);
    assert!(
        format!("{with_fallback:?}").contains("fallback: true"),
        "{with_fallback:?}"
    );

    let mut open_jet = DetectingPort::new();
    open_jet
        .open(&fixture("testV2000.mdb"), None)
        .expect("open the jet fixture");
    assert!(
        format!("{open_jet:?}").contains("jet"),
        "the session's driver id shows: {open_jet:?}"
    );
}

/// The fallback opens what the probes cannot: a `*.picky` file routes to
/// the registered driver, the session answers to it, and a following
/// built-in open takes the session back.
#[test]
fn the_fallback_opens_what_the_probes_cannot() {
    let file = Scratch(scratch_path("mystery.picky"));
    std::fs::write(&file.0, "whatever").expect("write the marker file");
    let mut port = DetectingPort::new().with_fallback(picky_factory);

    let opened = port.open(&file.0, None).expect("the fallback claims it");
    assert_eq!(opened.format, DatabaseFormat::Csv);
    assert_eq!(port.driver_id().as_str(), "picky", "the session answers");
    let caps = port.capabilities();
    assert!(caps.read_tables, "the picky session's declared set holds");
    assert!(!caps.write_rows, "declared absent: write_rows");
    assert_eq!(caps.autonumber, Support::No);

    // A built-in file takes the session back from the fallback.
    port.open(&fixture("testV2000.mdb"), None)
        .expect("open the jet fixture");
    assert_eq!(port.driver_id().as_str(), "jet");

    port.close();
    assert_eq!(port.driver_id().as_str(), "detecting");
}

/// An unknown file names the path, the expected source families *and*
/// the fallback's verdict in one renderable message (L9) — the hint
/// never regresses behind the fallback's rejection.
#[test]
fn an_unknown_file_names_the_family_hint_and_the_fallback_reason() {
    let file = Scratch(scratch_path("stranger.bin"));
    std::fs::write(&file.0, "definitely not a database").expect("write the stranger");

    let mut port = DetectingPort::new().with_fallback(picky_factory);
    let error = port
        .open(&file.0, None)
        .expect_err("the stranger must fail everywhere");
    let message = error.to_string();
    assert!(
        message.contains("expected a Jet/ACE header"),
        "the family hint survives: {message}"
    );
    assert!(
        message.contains("the registered fallback declined it too"),
        "the combined message marks the fallback verdict: {message}"
    );
    assert!(
        message.contains("not a picky source"),
        "the fallback's own reason is kept: {message}"
    );
    assert!(
        message.contains("stranger.bin"),
        "the path is named: {message}"
    );

    // Without a fallback the sniff's precise reason stays untouched.
    let mut plain = DetectingPort::new();
    let error = plain
        .open(&file.0, None)
        .expect_err("no fallback keeps the plain sniff error");
    assert!(
        !error.to_string().contains("fallback"),
        "no fallback is mentioned: {error}"
    );
}

/// `create_database` serves built-in formats itself and routes foreign
/// formats to the registered fallback — or refuses with the family
/// hint when none is registered.
#[test]
fn create_database_routes_foreign_formats_or_refuses() {
    let dir = Scratch(scratch_path("pickydb"));
    let mut routed = DetectingPort::new().with_fallback(picky_factory);
    routed
        .create_database(&dir.0, DatabaseFormat::Csv)
        .expect("the fallback serves the foreign format");
    assert!(dir.0.is_dir(), "the fallback created its database");

    let mut plain = DetectingPort::new();
    let error = plain
        .create_database(&scratch_path("nope"), DatabaseFormat::Csv)
        .expect_err("without a fallback the foreign format refuses");
    assert!(
        matches!(&error, PortError::Unsupported { reason } if reason.contains("Csv")),
        "{error:?}"
    );

    // Built-in formats still route to their own drivers.
    let jet = Scratch(scratch_path("fresh.mdb"));
    plain
        .create_database(&jet.0, DatabaseFormat::Jet4)
        .expect("jet4 creation is built-in");
    assert!(jet.0.exists());
}

/// The inventory calls answer the "no database is open" refusal while
/// nothing is open — precise, not silent.
#[test]
fn unopened_inventory_calls_answer_no_database() {
    let mut port = DetectingPort::new();
    let assert_no_db = |error: PortError, what: &str| {
        assert!(
            matches!(&error, PortError::Unsupported { reason } if reason.contains("no database is open")),
            "{what}: {error:?}"
        );
    };
    assert_no_db(
        port.restore_row("t", &[FieldValue::Null])
            .expect_err("restore_row"),
        "restore_row",
    );
    let relation = Relationship {
        table: "orders".to_owned(),
        column: "customer".to_owned(),
        related_table: "customers".to_owned(),
        related_column: "id".to_owned(),
    };
    assert_no_db(
        port.create_relationship(&relation)
            .expect_err("create_relationship"),
        "create_relationship",
    );
    assert_no_db(
        port.delete_relationship(&relation)
            .expect_err("delete_relationship"),
        "delete_relationship",
    );
    assert_no_db(
        port.alter_table(
            "t",
            SchemaChange::DropField {
                field: "x".to_owned(),
            },
        )
        .expect_err("alter_table"),
        "alter_table",
    );
    assert_no_db(
        port.save_as(Path::new("copy.mdb"), None)
            .expect_err("save_as"),
        "save_as",
    );
    assert_no_db(
        port.change_password(Some("s"))
            .expect_err("change_password"),
        "change_password",
    );
    // The read-side inventories stay empty instead of failing.
    assert!(port.relationships().expect("empty").is_empty());
    assert!(port.native_queries().expect("empty").is_empty());
    assert!(port.macros().expect("empty").is_empty());
    assert!(port.vba_project().expect("none").is_none());
}

/// The registry is data (M23): a composed core's snapshot lists the
/// built-in probes plus the registered fallback with ids, fixed families
/// and the offline capability summaries — the pre-flight surface for
/// agents, additive next to `templates`.
#[test]
fn the_composed_snapshot_lists_the_driver_inventory() {
    use datacraft_core::core::Core;
    use datacraft_core::port::DatabaseFormat;

    let core = Core::new(Box::new(DetectingPort::new().with_fallback(picky_factory)));
    let snapshot = core.inspect();
    assert_eq!(snapshot.drivers.len(), 3, "{:?}", snapshot.drivers);

    let jet = &snapshot.drivers[0];
    assert_eq!(jet.id.as_str(), "jet");
    assert_eq!(jet.format, Some(DatabaseFormat::Jet4));
    assert!(jet.capabilities.write_rows, "the offline reference set");

    let sqlite = &snapshot.drivers[1];
    assert_eq!(sqlite.id.as_str(), "sqlite");
    assert_eq!(sqlite.format, Some(DatabaseFormat::Sqlite));

    let picky = &snapshot.drivers[2];
    assert_eq!(picky.id.as_str(), "picky", "the fallback is inventoried");
    assert_eq!(
        picky.format,
        Some(DatabaseFormat::Csv),
        "PickyFallback names its family via source_format"
    );
    assert!(!picky.capabilities.write_rows, "the offline declared set");

    // A composition without a fallback inventories the built-ins only.
    let plain = Core::new(Box::new(DetectingPort::new()));
    assert_eq!(plain.inspect().drivers.len(), 2);
}
