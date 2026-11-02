//! Composite driver registry (phase 30, item M17; ADR-7): the detecting
//! port reports the *open driver's* identity and capabilities instead of
//! generic answers, and inventory calls come from the session — no
//! hardcoded per-backend arms. RED state: `DetectingPort` does not
//! override the capability reporting yet — it answers the optimistic
//! trait defaults ("generic"/full()) and hardcodes the SQLite inventory
//! arms.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, DriverId, StoragePort, Support};
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-registry-{}-{n}-{stem}",
        std::process::id()
    ))
}

fn sqlite_database() -> PathBuf {
    let path = scratch_path("data.sqlite");
    let _ = std::fs::remove_file(&path);
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch("CREATE TABLE notes (body TEXT);")
        .expect("populate scratch database");
    path
}

/// A Jet session surfaces the jet driver's identity and per-session
/// capabilities through the composite.
#[test]
fn the_composite_reports_the_jet_session_identity_and_capabilities() {
    let mut port = DetectingPort::new();
    port.open(&fixture("testV2000.mdb"), None)
        .expect("open the Jet4 fixture");
    assert_eq!(port.driver_id(), DriverId::new("jet"));
    let caps = port.capabilities();
    assert!(caps.atomic_batch, "the jet driver's atomicity answer");
    assert!(caps.passwords);
    assert_eq!(caps.autonumber, Support::Yes);
    assert_eq!(caps.native_queries, Support::ReadOnly);
}

/// A SQLite session surfaces the sqlite driver — including its `No`
/// inventories, which used to be hardcoded in the dispatcher.
#[test]
fn the_composite_reports_the_sqlite_session_identity_and_capabilities() {
    let path = sqlite_database();
    let mut port = DetectingPort::new();
    port.open(&path, None).expect("open the SQLite file");
    assert_eq!(port.driver_id(), DriverId::new("sqlite"));
    let caps = port.capabilities();
    assert!(!caps.atomic_batch);
    assert!(!caps.passwords);
    assert_eq!(caps.native_queries, Support::No);
    assert_eq!(caps.macros, Support::No);
    assert_eq!(caps.vba, Support::No);
    assert!(port.macros().expect("inventory").is_empty());
    assert!(port.vba_project().expect("project").is_none());
    let _ = std::fs::remove_file(&path);
}

/// While nothing is open the composite speaks for itself; creation stays
/// session-independent through the format routing.
#[test]
fn the_composite_answers_itself_while_nothing_is_open() {
    let mut port = DetectingPort::new();
    assert_eq!(port.driver_id(), DriverId::new("detecting"));
    assert!(port.capabilities().create_database);
    let path = scratch_path("created.mdb");
    let _ = std::fs::remove_file(&path);
    port.create_database(&path, DatabaseFormat::Jet4)
        .expect("create through the composite");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}
