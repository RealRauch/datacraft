//! Atomic persistence contract (step 34.5, item H47).
//!
//! Every database and sidecar save goes through the sibling-temp pattern:
//! a failed or interrupted write must leave the live file byte-identical,
//! and a create must never leave a partial file.

use std::path::PathBuf;

use datacraft_core::atomic_write;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-atomic-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.join("target.bin")
}

fn temp_of(path: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp", path.display()))
}

#[test]
fn replacing_swaps_the_file_and_leaves_no_temp() {
    let path = scratch("replace");
    std::fs::write(&path, b"old").expect("seed");
    atomic_write::write_replacing(&path, b"new").expect("replace");
    assert_eq!(std::fs::read(&path).expect("read"), b"new");
    assert!(!temp_of(&path).exists(), "the sibling temp must be gone");
}

#[test]
fn a_failed_replace_keeps_the_original_intact() {
    let path = scratch("failed");
    std::fs::write(&path, b"old").expect("seed");
    std::fs::create_dir_all(temp_of(&path)).expect("block the temp path");
    let err = atomic_write::write_replacing(&path, b"new").expect_err("temp write must fail");
    assert!(!err.to_string().is_empty());
    assert_eq!(
        std::fs::read(&path).expect("read"),
        b"old",
        "the live file must never be truncated by a failed save"
    );
}

#[test]
fn creating_writes_a_new_file_and_refuses_an_existing_target() {
    let path = scratch("new");
    atomic_write::write_new(&path, b"data").expect("create");
    assert_eq!(std::fs::read(&path).expect("read"), b"data");

    let err = atomic_write::write_new(&path, b"other").expect_err("existing target");
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&path).expect("read"), b"data");
}
