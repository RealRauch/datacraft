//! Native-query decode-error contract on the self-created programmability
//! fixture (step 23.3, item M10; fixture replaced by H50).
//!
//! `programmability.accdb` carries `MSysQueries` attribute code 12 — the
//! multi-value expansion of the `Tags` column (the bound form's shadow
//! query uses it). The port must answer a precise `Unsupported` instead of
//! an empty list — the core snapshot then surfaces it.

use std::path::{Path, PathBuf};

use datacraft_core::port::{PortError, StoragePort};
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn the_multi_value_attribute_12_is_a_precise_unsupported() {
    let mut port = Jet4Port::new();
    port.open(&fixture("programmability.accdb"), None)
        .expect("fixture opens");
    let error = port
        .native_queries()
        .expect_err("attribute 12 is outside the decoded subset");
    let PortError::Unsupported { reason } = error else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert!(reason.contains("attribute code 12"), "{reason}");
}

#[test]
fn the_plain_query_fixture_decodes_without_error() {
    let mut port = Jet4Port::new();
    port.open(&fixture("queryTestV2000.mdb"), None)
        .expect("fixture opens");
    let queries = port.native_queries().expect("the query fixture decodes");
    assert!(!queries.is_empty());
}
