//! The deterministic canonical-database generators (step 61.1, item L35):
//! every generator is byte-deterministic per input state, the generated
//! databases reopen through the port with their catalog, typed kinds and
//! index metadata — and the fixtures stay untouched (the pins and the
//! oracle keep their own suites).

mod support;

use std::path::PathBuf;

use datacraft_core::port::StoragePort;
use datacraft_core::schema::{FieldKind, IndexMode};
use datacraft_jet::port_adapter::Jet4Port;
use support::generators::{generated_empty, generated_indexed, generated_typed};

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "datacraft-generated-{name}-{}",
        std::process::id()
    ))
}

/// Every generator is byte-deterministic per input state (L35
/// acceptance): two runs write identical bytes.
#[test]
fn the_generators_are_byte_deterministic() {
    let dir = scratch("deterministic");
    for generator in [generated_empty, generated_typed, generated_indexed] {
        let first = std::fs::read(generator(&dir, "first.mdb")).expect("first run writes");
        let second = std::fs::read(generator(&dir, "second.mdb")).expect("second run writes");
        assert_eq!(
            first, second,
            "the generator is byte-deterministic per input state"
        );
    }
}

/// The generated typed database reopens with its catalog and the mapped
/// kinds.
#[test]
fn the_generated_typed_database_reopens() {
    let dir = scratch("typed");
    let path = generated_typed(&dir, "typed.mdb");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("the generated file opens");
    let table = port
        .tables()
        .into_iter()
        .find(|table| table.name == "Typed")
        .expect("the typed table is catalogued");
    assert_eq!(table.fields.len(), 5);
    assert!(matches!(
        table.fields[1].kind,
        FieldKind::Number(_)
    ));
    assert!(matches!(
        table.fields[4].kind,
        FieldKind::Boolean { .. }
    ));
}

/// The generated indexed database reopens with the index metadata the
/// domain carries.
#[test]
fn the_generated_indexed_database_reopens_with_the_index() {
    let dir = scratch("indexed");
    let path = generated_indexed(&dir, "indexed.mdb");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("the generated file opens");
    let table = port
        .tables()
        .into_iter()
        .find(|table| table.name == "Indexed")
        .expect("the indexed table is catalogued");
    assert_eq!(table.fields[0].properties.indexed, IndexMode::NoDuplicates);
}

/// Different generators produce different bytes — the determinism is not
/// trivial.
#[test]
fn different_generators_produce_different_bytes() {
    let dir = scratch("distinct");
    let empty = std::fs::read(generated_empty(&dir, "empty.mdb")).expect("empty");
    let typed = std::fs::read(generated_typed(&dir, "typed.mdb")).expect("typed");
    let indexed = std::fs::read(generated_indexed(&dir, "indexed.mdb")).expect("indexed");
    assert_ne!(empty, typed, "the typed table changes the bytes");
    assert_ne!(typed, indexed, "the index changes the bytes");
}
