//! Atomic persistence contract (steps 34.5 and 80.1, items H47 and H60).
//!
//! Every database and sidecar save goes through the staging pattern:
//! a failed or interrupted write must leave the live file byte-identical,
//! and a create must never leave a partial file. H60 adds the durability
//! and concurrency guarantees: unique staging names, a flush before the
//! rename, bounded Windows rename retries, symlink resolution, mode
//! preservation and a fault-injection seam for the staging order.

use std::path::{Path, PathBuf};

use datacraft_core::atomic_write::{self, StagingEvent};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-atomic-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.join("target.bin")
}

/// Every leftover staging file (`.tmp`) currently in `path`'s directory.
fn staging_files(target: &Path) -> Vec<PathBuf> {
    let directory = target.parent().expect("target has a parent");
    let mut files: Vec<PathBuf> = std::fs::read_dir(directory)
        .expect("scratch directory is readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|extension| extension == "tmp")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn replacing_swaps_the_file_and_leaves_no_staging_file() {
    let path = scratch("replace");
    std::fs::write(&path, b"old").expect("seed");
    atomic_write::write_replacing(&path, b"new").expect("replace");
    assert_eq!(std::fs::read(&path).expect("read"), b"new");
    assert!(
        staging_files(&path).is_empty(),
        "the staging file must be gone after a successful replace"
    );
}

#[test]
fn the_observed_write_records_the_stage_flush_rename_order() {
    let path = scratch("observed");
    std::fs::write(&path, b"old").expect("seed");
    let mut stages = Vec::new();
    atomic_write::write_replacing_observed(&path, b"new", &mut |event| {
        stages.push(event.clone())
    })
    .expect("replace");
    assert_eq!(stages.len(), 3, "staged, flushed and renamed in order");
    let staged = match &stages[0] {
        StagingEvent::Staged(staging) => staging.clone(),
        other => panic!("the first stage must be the staging file, got {other:?}"),
    };
    assert_eq!(
        staged.parent(),
        path.parent(),
        "the staging file is a sibling of the target"
    );
    assert_ne!(staged, path, "the staging file must not be the target");
    assert_eq!(stages[1], StagingEvent::Flushed);
    assert_eq!(stages[2], StagingEvent::Renamed);
    assert_eq!(std::fs::read(&path).expect("read"), b"new");
}

#[test]
fn a_failed_rename_records_the_stages_it_reached_and_keeps_the_target() {
    // A non-empty directory at the target path cannot be replaced by a
    // file rename, so the write fails after staging and flushing.
    let path = scratch("rename");
    std::fs::create_dir_all(path.join("keep")).expect("make the target a directory");
    let mut stages = Vec::new();
    let error = atomic_write::write_replacing_observed(&path, b"new", &mut |event| {
        stages.push(event.clone())
    })
    .expect_err("rename must fail");
    assert!(!error.to_string().is_empty());
    assert_eq!(
        stages.last(),
        Some(&StagingEvent::Flushed),
        "the rename stage must never be reported when it did not happen"
    );
    assert!(path.is_dir(), "the original target must stay untouched");
    assert!(
        staging_files(&path).is_empty(),
        "the staging file must be cleaned up after the failed rename"
    );
}

#[test]
fn staging_paths_are_unique_across_calls() {
    let path = scratch("unique");
    let first = atomic_write::staging_path(&path);
    let second = atomic_write::staging_path(&path);
    assert_ne!(first, second, "two writers must never share one staging file");
    assert_eq!(
        first.parent(),
        path.parent(),
        "the staging file is a sibling of the target"
    );
}

#[test]
fn concurrent_writers_never_share_a_staging_file() {
    let path = scratch("concurrent");
    std::fs::write(&path, b"seed").expect("seed");
    let handles: Vec<_> = (0..4)
        .map(|writer| {
            let path = path.clone();
            std::thread::spawn(move || {
                atomic_write::write_replacing(&path, format!("writer-{writer}").as_bytes())
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread").expect("replace");
    }
    let final_content = std::fs::read(&path).expect("read");
    let written: Vec<Vec<u8>> = (0..4)
        .map(|writer| format!("writer-{writer}").into_bytes())
        .collect();
    assert!(
        written.contains(&final_content),
        "one complete write must win, got {final_content:?}"
    );
    assert!(
        staging_files(&path).is_empty(),
        "every staging file must be cleaned up after the writes"
    );
}

#[test]
fn a_stale_orphan_staging_file_does_not_block_a_new_write() {
    // A crash between staging and the rename leaves the target
    // byte-identical and at most one orphan staging file behind; later
    // saves must ignore it.
    let path = scratch("orphan");
    std::fs::write(&path, b"old").expect("seed");
    let orphan = path.with_file_name("target.bin.999-0.tmp");
    std::fs::write(&orphan, b"orphan").expect("plant the orphan");
    atomic_write::write_replacing(&path, b"new").expect("replace past the orphan");
    assert_eq!(std::fs::read(&path).expect("read"), b"new");
    assert_eq!(
        staging_files(&path),
        vec![orphan],
        "only the planted orphan may remain"
    );
}

#[test]
fn a_read_only_target_still_refuses_the_replacement() {
    let path = scratch("readonly");
    std::fs::write(&path, b"old").expect("seed");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).expect("set read-only");
    let error = atomic_write::write_replacing(&path, b"new").expect_err("read-only refusal");
    assert!(
        error.to_string().contains("read-only"),
        "the refusal must name the read-only target, got {error}"
    );
    assert_eq!(std::fs::read(&path).expect("read"), b"old");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).expect("clear read-only");
}

#[test]
fn creating_writes_a_new_file_and_refuses_an_existing_target() {
    let path = scratch("new");
    atomic_write::write_new(&path, b"data").expect("create");
    assert_eq!(std::fs::read(&path).expect("read"), b"data");

    let error = atomic_write::write_new(&path, b"other").expect_err("existing target");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&path).expect("read"), b"data");
}

#[cfg(unix)]
mod unix_only {
    use super::*;

    #[test]
    fn a_symlinked_target_is_replaced_through_the_link() {
        let path = scratch("symlink");
        let real = path.with_file_name("real.bin");
        std::fs::write(&real, b"old").expect("seed the real file");
        std::os::unix::fs::symlink(&real, &path).expect("link");
        atomic_write::write_replacing(&path, b"new").expect("replace through the link");
        assert_eq!(
            std::fs::read(&real).expect("read"),
            b"new",
            "the link's destination must be replaced"
        );
        let metadata = std::fs::symlink_metadata(&path).expect("link metadata");
        assert!(
            metadata.file_type().is_symlink(),
            "the link itself must survive the replacement"
        );
    }

    #[test]
    fn mode_of_the_replaced_file_is_restored() {
        let path = scratch("mode");
        std::fs::write(&path, b"old").expect("seed");
        std::os::unix::fs::PermissionsExt::set_mode(
            &mut std::fs::metadata(&path).expect("metadata").permissions(),
            0o640,
        );
        std::fs::set_permissions(
            &path,
            std::os::unix::fs::PermissionsExt::from_mode(0o640),
        )
        .expect("set the mode");
        atomic_write::write_replacing(&path, b"new").expect("replace");
        let mode = std::os::unix::fs::PermissionsExt::mode(
            &std::fs::metadata(&path).expect("metadata").permissions(),
        );
        assert_eq!(mode & 0o777, 0o640, "the replaced file keeps the mode");
    }
}

#[cfg(windows)]
#[test]
fn a_windows_file_lock_fails_the_write_which_then_succeeds_after_release() {
    let path = scratch("winlock");
    std::fs::write(&path, b"old").expect("seed");
    // A plain open handle holds the target without FILE_SHARE_DELETE, so
    // the rename hits the OS sharing/lock error the bounded retry covers.
    let lock = std::fs::File::open(&path).expect("lock the target");
    let error = atomic_write::write_replacing(&path, b"new").expect_err("rename must hit the lock");
    assert!(!error.to_string().is_empty());
    assert!(
        staging_files(&path).is_empty(),
        "the staging file must be cleaned up after the retry budget is spent"
    );
    drop(lock);
    atomic_write::write_replacing(&path, b"new").expect("the retry budget frees the path");
    assert_eq!(std::fs::read(&path).expect("read"), b"new");
}
