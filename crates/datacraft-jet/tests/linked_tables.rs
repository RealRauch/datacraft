//! Linked-table inventory (phase 100, step 100.1; item M131; roadmap §9
//! stage 0).
//!
//! MSysObjects link records (Type 4 = ODBC-linked, Type 6 = Access-file
//! linked) were silently invisible: the catalog skipped every non-TDEF
//! row. The inventory decodes them read-only — name, kind, external
//! database or connect string, foreign name and flags — and a connect
//! string arrives with its credentials redacted. Opening a file never
//! dials out and never writes.

use datacraft_core::port::{LinkKind, StoragePort};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn opened(name: &str) -> Jet4Port {
    let mut port = Jet4Port::new();
    port.open(&fixture(name), None).expect("fixture opens");
    port
}

#[test]
fn an_access_file_link_reports_its_target_and_foreign_name() {
    let port = opened("linkerTestV2007.accdb");
    let links = port.linked_tables().expect("link inventory");
    assert_eq!(links.len(), 1, "{links:?}");
    let link = &links[0];
    assert_eq!(link.name, "Table2");
    assert_eq!(link.kind, LinkKind::AccessFile);
    assert_eq!(
        link.source.as_deref(),
        Some("Z:\\jackcess_test\\linkeeTest.accdb"),
        "the DATABASE column names the external file"
    );
    assert_eq!(link.connect, None, "a file link carries no connect string");
    assert_eq!(link.foreign_name.as_deref(), Some("Table1"));
    assert_eq!(link.flags, 2_097_152);
}

#[test]
fn an_odbc_link_reports_its_connect_string_without_credentials() {
    let port = opened("odbcLinkerTestV2007.accdb");
    let links = port.linked_tables().expect("link inventory");
    assert_eq!(links.len(), 1, "{links:?}");
    let link = &links[0];
    assert_eq!(link.name, "Ordrar");
    assert_eq!(link.kind, LinkKind::Odbc);
    assert_eq!(link.source, None, "an ODBC link names no external file");
    assert_eq!(link.foreign_name.as_deref(), Some("dbo.Ordrar"));
    assert_eq!(link.flags, 537_919_488);
    let connect = link.connect.as_deref().expect("a connect string");
    assert!(
        connect.starts_with("DSN=Magnapinna;"),
        "the non-secret fragments stay readable: {connect}"
    );
    assert!(
        connect.contains("DATABASE=safexit"),
        "the remote database name survives: {connect}"
    );
    assert!(
        connect.contains("PWD=***"),
        "the password is redacted: {connect}"
    );
    assert!(
        !connect.contains("DummyPassword"),
        "the secret never reaches the caller: {connect}"
    );
}

#[test]
fn a_link_never_becomes_a_table() {
    let port = opened("linkerTestV2007.accdb");
    let catalog = port.tables();
    let tables: Vec<&str> = catalog.iter().map(|def| def.name.as_str()).collect();
    assert!(
        !tables.contains(&"Table2"),
        "the link has no TDEF page and stays out of the table catalog: {tables:?}"
    );
    let links = port.linked_tables().expect("inventory");
    assert!(links.iter().any(|link| link.name == "Table2"), "{links:?}");
}

#[test]
fn a_link_free_file_reports_an_empty_inventory() {
    let port = opened("testV2000.mdb");
    assert_eq!(port.linked_tables().expect("inventory"), Vec::new());
}

/// The self-built fixture (`tools/fixture-build/links.ps1`) is a second,
/// independent Access writer: same record shape, and its target path does
/// not resolve — the inventory still reports it completely, which is the
/// whole point of stage 0.
#[test]
fn the_self_built_fixture_reports_the_same_record_shape() {
    let port = opened("linkedTablesV2007.accdb");
    let links = port.linked_tables().expect("inventory");
    assert_eq!(links.len(), 1, "{links:?}");
    let link = &links[0];
    assert_eq!(link.name, "Remote_Link");
    assert_eq!(link.kind, LinkKind::AccessFile);
    assert_eq!(link.foreign_name.as_deref(), Some("Remote"));
    let source = link.source.as_deref().expect("a DATABASE path");
    assert!(source.ends_with("linkee.accdb"), "{source}");
    assert!(
        !std::path::Path::new(source).exists(),
        "the target is gone by construction: {source}"
    );
    assert_eq!(
        link.flags, 2_097_152,
        "two independent Access writers agree on the file-link flags"
    );
    let catalog = port.tables();
    let names: Vec<&str> = catalog.iter().map(|def| def.name.as_str()).collect();
    assert!(names.contains(&"Local"), "{names:?}");
    assert!(!names.contains(&"Remote_Link"), "{names:?}");
}

#[test]
fn opening_and_reading_the_inventory_leaves_the_file_byte_identical() {
    let path = fixture("odbcLinkerTestV2007.accdb");
    let before = std::fs::read(&path).expect("read the fixture");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("fixture opens");
    let links = port.linked_tables().expect("inventory");
    assert_eq!(links.len(), 1);
    port.close();
    assert_eq!(
        std::fs::read(&path).expect("read the fixture again"),
        before,
        "the inventory is read-only by construction"
    );
}
