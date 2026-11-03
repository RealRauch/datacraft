//! Atomic file persistence (step 34.5, item H47).
//!
//! Database and sidecar saves follow the settings-file pattern: write a
//! sibling temporary file, then rename it over the target. A crash, a full
//! disk or a failed flush in the window leaves the live file byte-identical
//! instead of truncated. Creation uses an exclusive `create_new` handle and
//! removes the partial file on failure.

use std::path::{Path, PathBuf};

/// Sibling temp path of `path` (`<name>.tmp`).
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Replaces `path` with `bytes` through a sibling temporary file.
///
/// A failure while writing the temp leaves the target untouched and removes
/// the temp; a failure while renaming keeps the target and removes the temp.
pub fn write_replacing(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // A read-only target must refuse the replacement on every platform:
    // POSIX rename only needs write access to the directory, so without
    // this check the read-only flag would silently lose its protection
    // for files written by the Jet write-through (T8 root cause, F9).
    // Windows rename-replace enforces the flag itself; a missing target
    // has no flag and proceeds.
    if std::fs::metadata(path)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(false)
    {
        return Err(std::io::Error::other("target file is read-only"));
    }
    let temp = temp_path(path);
    if let Err(error) = std::fs::write(&temp, bytes) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

/// Creates `path` with `bytes`, refusing an existing target.
///
/// The open uses `create_new`, so two concurrent creators cannot overwrite
/// each other; a failed write removes the partial file.
pub fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}
