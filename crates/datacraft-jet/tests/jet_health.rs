//! Pre-open health facts for Jet/ACE (phase 95, step 95.1; item M126).
//!
//! The health check is read-only and bounded: the header decode already
//! rejects unusable files, this layer reports the damage a readable file
//! still carries (unknown page types, a broken usage map) as a precise
//! `health` reason without blocking the open. Facts ride the snapshot so
//! every client sees the same file truth.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::StoragePort;
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

/// A private copy of a pinned fixture (fixtures are immutable evidence).
fn scratch_copy(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "datacraft-jet-health-{}-{n}-{name}",
        std::process::id()
    ));
    std::fs::copy(fixture(name), &path).expect("copy the fixture");
    path
}

fn fact<'a>(facts: &'a datacraft_core::port::SourceFacts, name: &str) -> Option<&'a str> {
    facts
        .facts
        .iter()
        .find(|fact| fact.name == name)
        .map(|fact| fact.value.as_str())
}

#[test]
fn a_healthy_jet4_fixture_reports_facts_and_no_finding() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2000.mdb"), None)
        .expect("fixture must open");
    let facts = port.source_facts();
    assert_eq!(facts.health, None, "a healthy file has no finding");
    assert_eq!(fact(&facts, "jet_version"), Some("Jet4"));
    assert_eq!(fact(&facts, "page_size"), Some("4096"));
    assert_eq!(fact(&facts, "encrypted"), Some("no"));
    let pages: usize = fact(&facts, "page_count")
        .expect("page count fact")
        .parse()
        .expect("numeric page count");
    assert!(pages > 1, "fixture spans several pages");
    let file_bytes: u64 = fact(&facts, "file_bytes")
        .expect("file size fact")
        .parse()
        .expect("numeric file size");
    assert_eq!(file_bytes, (pages * 4096) as u64);
    let code_page: u16 = fact(&facts, "code_page")
        .expect("code page fact")
        .parse()
        .expect("numeric code page");
    assert!(code_page > 0);
}

#[test]
fn an_ace_fixture_names_the_ace_version() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2010.accdb"), None)
        .expect("ACE fixture must open");
    let facts = port.source_facts();
    assert_eq!(facts.health, None, "a healthy file has no finding");
    assert_eq!(fact(&facts, "jet_version"), Some("ACE"));
    assert_eq!(fact(&facts, "page_size"), Some("4096"));
}

#[test]
fn an_unknown_page_type_reports_a_locatable_finding_without_blocking() {
    let path = scratch_copy("testV2000.mdb");
    let mut bytes = std::fs::read(&path).expect("read the copy");
    // Corrupt an unused page (type byte 0x00) in the middle of the file:
    // no catalog or TDEF page, so the open still succeeds.
    let page = (2..bytes.len() / 4096)
        .find(|index| bytes[index * 4096] == 0x00)
        .expect("the fixture carries an unused page");
    bytes[page * 4096] = 0x7F;
    std::fs::write(&path, &bytes).expect("write the damaged copy");

    let mut port = Jet4Port::new();
    port.open(&path, None)
        .expect("a damaged page must not block the open");
    let facts = port.source_facts();
    let health = facts.health.expect("the damage must be reported");
    assert!(
        health.contains(&format!("page {page}")),
        "the finding names the page: {health}"
    );
    assert!(
        health.to_lowercase().contains("0x7f"),
        "the finding names the byte: {health}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn close_clears_the_facts() {
    let mut port = Jet4Port::new();
    port.open(&fixture("testV2000.mdb"), None)
        .expect("fixture must open");
    assert!(!port.source_facts().facts.is_empty());
    port.close();
    assert_eq!(port.source_facts(), Default::default());
}
