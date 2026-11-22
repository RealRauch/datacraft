//! Access lock-file detection (step 53.1, item M59).
//!
//! A sibling `.ldb` (Jet) or `.laccdb` (ACE) means Access may hold the
//! database open; the session opens read-only. Detection is
//! existence-only — the lock protocol's contents are clean-room-unclear
//! and out of scope. DataCraft never creates or deletes lock files.

use std::path::{Path, PathBuf};

/// The lock file beside `path`, when one exists.
pub(crate) fn detect_lock_file(path: &Path) -> Option<PathBuf> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let lock_extension = match extension.as_str() {
        "mdb" => "ldb",
        "accdb" => "laccdb",
        _ => return None,
    };
    let lock = path.with_extension(lock_extension);
    lock.exists().then_some(lock)
}
