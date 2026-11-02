//! Deterministic sample contracts (phase 29, step 29.3, item H44).
//!
//! The shipped `samples/` set is the byte-exact output of the generator:
//! `check` regenerates into a temp directory and compares, which is what
//! `make_samples --check` runs and what the committed-samples test below
//! enforces on every CI run. RED state: the `samples` module does not
//! exist yet — the suite fails to compile until it lands.

use std::path::{Path, PathBuf};

use datacraft_core::port::{DatabaseFormat, StoragePort, RowRange};
use datacraft_storage::detecting::DetectingPort;
use datacraft_storage::samples;

fn workspace_samples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
}

fn scratch_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("datacraft-samples-{name}-{}", std::process::id()))
}

#[test]
fn the_shipped_samples_match_the_deterministic_generator() {
    let dir = workspace_samples();
    assert!(dir.is_dir(), "samples/ ships in the workspace");
    samples::check(&dir).expect("the committed samples are the deterministic output");
}

#[test]
fn check_detects_a_tampered_sample() {
    let dir = scratch_dir("tamper");
    let _ = std::fs::remove_dir_all(&dir);
    samples::generate_all(&dir).expect("generate into the temp dir");
    let mut blank = std::fs::read(dir.join("blank.mdb")).expect("blank exists");
    blank[0x42] ^= 0xFF;
    std::fs::write(dir.join("blank.mdb"), &blank).expect("tamper");
    let message = samples::check(&dir).expect_err("a tampered sample fails the check");
    assert!(message.contains("blank.mdb"), "{message}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_generated_sample_opens_with_its_expected_tables() {
    let dir = scratch_dir("open");
    let _ = std::fs::remove_dir_all(&dir);
    samples::generate_all(&dir).expect("generate");

    let expected: [(&str, DatabaseFormat, &[(&str, usize)]); 5] = [
        ("blank.mdb", DatabaseFormat::Jet4, &[]),
        ("contacts.mdb", DatabaseFormat::Jet4, &[("Contacts", 4)]),
        (
            "inventory.mdb",
            DatabaseFormat::Jet4,
            &[("Categories", 4), ("Items", 6)],
        ),
        (
            "bike_shop.mdb",
            DatabaseFormat::Jet4,
            &[("Bikes", 6), ("Customers", 4), ("ServiceOrders", 3)],
        ),
        (
            "bike_shop.sqlite",
            DatabaseFormat::Sqlite,
            &[("Bikes", 6), ("Customers", 4), ("ServiceOrders", 3)],
        ),
    ];
    for (file, format, tables) in expected {
        let mut port = DetectingPort::new();
        let opened = port
            .open(&dir.join(file), None)
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(opened.format, format, "{file}");
        let names: Vec<String> = port.tables().into_iter().map(|t| t.name).collect();
        for (table, rows) in tables {
            assert!(
                names.iter().any(|name| name.eq_ignore_ascii_case(table)),
                "{file}: {table} missing in {names:?}"
            );
            let count = port
                .read_rows(table, RowRange { start: 0, len: 100 })
                .unwrap_or_else(|e| panic!("{file}/{table}: {e}"))
                .len();
            assert_eq!(count, *rows, "{file}/{table}");
        }
    }

    // The demo sidecar ships next to its database and names the demo form.
    let sidecar = std::fs::read_to_string(dir.join("bike_shop.datacraft"))
        .expect("the demo sidecar ships");
    assert!(sidecar.contains("Bike Finder"), "the demo form ships: {sidecar}");
    let _ = std::fs::remove_dir_all(&dir);
}
