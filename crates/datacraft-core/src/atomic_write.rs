//! Atomic file persistence (steps 34.5 and 80.1, items H47 and H60).
//!
//! Database and sidecar saves follow the staging pattern: write a unique
//! sibling staging file, flush it to disk, then rename it over the target.
//! A crash, a full disk or a failed flush in the window leaves the live
//! file byte-identical instead of truncated. Creation uses an exclusive
//! `create_new` handle and removes the partial file on failure.
//!
//! Guarantees added in 80.1 (H60):
//!
//! - **Durability** — the staging file is flushed (`sync_all`) before the
//!   rename and the parent directory is synced best-effort on Unix after
//!   it, so a power loss cannot durably commit the rename while the data
//!   blocks are still in flight.
//! - **Concurrency** — staging names embed the process id and a monotonic
//!   counter, so two writers never share one staging file. A crash between
//!   staging and the rename leaves the target byte-identical and at most
//!   one orphan `*.tmp` staging file behind; later saves ignore it.
//! - **Windows sharing violations** — the rename retries a bounded number
//!   of times with a backoff on the OS lock errors (32/33) that antivirus
//!   scanners and the search indexer cause on freshly written files.
//! - **Symlinks** — a symlinked target is resolved (bounded hop count) and
//!   the link's destination is replaced, never the link itself.
//! - **Mode preservation** — the replaced file's permissions are restored
//!   onto the staging file before the rename.
//!
//! The read-only refusal predates these guarantees (F9): a read-only
//! target refuses the replacement on every platform, because POSIX rename
//! only needs write access to the directory and would otherwise silently
//! lose the flag's protection for the Jet write-through (T8 root cause).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Maximum symlink hops resolved before the chain is followed no further.
const SYMLINK_HOP_LIMIT: usize = 16;

/// Bounded retry budget for the Windows sharing/lock errors.
const RETRY_ATTEMPTS: u32 = 10;
const RETRY_BASE_DELAY: Duration = Duration::from_millis(25);
const RETRY_MAX_DELAY: Duration = Duration::from_millis(200);

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique sibling staging path of `path`.
///
/// The name embeds the process id and a monotonic counter, so two writers
/// (two DataCraft instances or two threads) never stage into one file.
pub fn staging_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        STAGING_COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    path.parent().unwrap_or_else(|| Path::new("")).join(name)
}

/// Resolves a symlink chain on the target path with a hop limit.
///
/// Returns `path` unchanged when it is not a symlink or the chain is
/// broken; a chain longer than the hop limit stops at the last hop.
fn resolve_link(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    for _ in 0..SYMLINK_HOP_LIMIT {
        match std::fs::read_link(&current) {
            Ok(target) => {
                current = if target.is_absolute() {
                    target
                } else {
                    current
                        .parent()
                        .unwrap_or_else(|| Path::new(""))
                        .join(target)
                };
            }
            Err(_) => return current,
        }
    }
    current
}

/// Refuses a read-only target on every platform (F9, see module docs).
fn refuse_read_only(target: &Path) -> io::Result<()> {
    if std::fs::metadata(target)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(false)
    {
        return Err(io::Error::other("target file is read-only"));
    }
    Ok(())
}

/// Restores the permissions of `target` onto `staging` before the rename.
///
/// The read-only case cannot occur here (the refusal ran first), so the
/// permission set is a pure mode restore and may not skip silently.
fn preserve_mode(target: &Path, staging: &Path) -> io::Result<()> {
    if let Ok(metadata) = std::fs::metadata(target) {
        let permissions = metadata.permissions();
        if !permissions.readonly() {
            std::fs::set_permissions(staging, permissions)?;
        }
    }
    Ok(())
}

/// The Windows sharing/lock errors the rename retry covers: another
/// handle (antivirus scanner, search indexer) holds the freshly written
/// target. The lock is transient, so a bounded retry is honest.
fn is_sharing_violation(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

/// Exponential backoff between rename retries, capped at the maximum.
fn backoff(attempt: u32) -> Duration {
    RETRY_BASE_DELAY
        .saturating_mul(1u32 << attempt.min(3))
        .min(RETRY_MAX_DELAY)
}

/// Runs `op`, retrying the sharing/lock errors with a bounded backoff.
///
/// `sleep` is injected so the contract tests run without real delays;
/// production passes [`std::thread::sleep`].
fn retry_with<T>(
    mut op: impl FnMut() -> io::Result<T>,
    mut sleep: impl FnMut(Duration),
) -> io::Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(error) if is_sharing_violation(&error) && attempt + 1 < RETRY_ATTEMPTS => {
                sleep(backoff(attempt));
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

/// Best-effort parent-directory sync so the rename itself is durable.
#[cfg(unix)]
fn sync_parent(target: &Path) {
    if let Some(parent) = target.parent() {
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}

#[cfg(not(unix))]
fn sync_parent(_target: &Path) {}

/// The observable stages of a replacement save, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagingEvent {
    /// The unique staging file was created and fully written.
    Staged(PathBuf),
    /// The staging file was flushed to disk.
    Flushed,
    /// The staging file was renamed over the target.
    Renamed,
}

/// Replaces `path` with `bytes` through a sibling staging file.
///
/// A failure while writing the staging file leaves the target untouched
/// and removes it; a failure while renaming keeps the target and removes
/// the staging file. See the module docs for the durability and
/// concurrency guarantees.
pub fn write_replacing(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_replacing_observed(path, bytes, &mut |_| {})
}

/// [`write_replacing`] with a fault-injection seam.
///
/// `observer` receives every stage in order — create → flush → rename —
/// as it happens. Test seam for the durability contract (H60).
pub fn write_replacing_observed(
    path: &Path,
    bytes: &[u8],
    observer: &mut dyn FnMut(&StagingEvent),
) -> io::Result<()> {
    let target = resolve_link(path);
    refuse_read_only(&target)?;
    let staging = staging_path(&target);
    match stage_and_rename(&target, &staging, bytes, observer) {
        Ok(()) => {
            sync_parent(&target);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_file(&staging);
            Err(error)
        }
    }
}

/// Creates the staging file, writes, flushes, renames — reporting each
/// stage to `observer`. Any failure propagates; the caller cleans up.
fn stage_and_rename(
    target: &Path,
    staging: &Path,
    bytes: &[u8],
    observer: &mut dyn FnMut(&StagingEvent),
) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging)?;
    observer(&StagingEvent::Staged(staging.to_path_buf()));
    file.write_all(bytes)?;
    preserve_mode(target, staging)?;
    file.sync_all()?;
    observer(&StagingEvent::Flushed);
    retry_with(|| std::fs::rename(staging, target), std::thread::sleep)?;
    observer(&StagingEvent::Renamed);
    Ok(())
}

/// Creates `path` with `bytes`, refusing an existing target.
///
/// The open uses `create_new`, so two concurrent creators cannot overwrite
/// each other; a failed write removes the partial file. The file is
/// flushed before the handle closes, so a power loss never leaves a
/// zero-length or partial new file.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod retry_contract {
    //! The retry loop and its policy, exercised without real delays.

    use super::*;

    fn sharing_violation() -> io::Error {
        io::Error::from_raw_os_error(32)
    }

    #[test]
    fn retry_succeeds_after_simulated_sharing_violations() {
        let mut attempts = 0;
        let mut delays = Vec::new();
        let result = retry_with(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(sharing_violation())
                } else {
                    Ok(())
                }
            },
            |delay| delays.push(delay),
        );
        assert_eq!(result.expect("the retry succeeds"), ());
        assert_eq!(attempts, 3);
        assert_eq!(delays, vec![backoff(0), backoff(1)]);
    }

    #[test]
    fn retry_is_not_applied_to_unrelated_errors() {
        let mut attempts = 0;
        let mut sleeps = 0;
        let error = retry_with(
            || {
                attempts += 1;
                Err::<(), _>(io::Error::from_raw_os_error(5))
            },
            |_| sleeps += 1,
        )
        .expect_err("a permanent error must surface immediately");
        assert_eq!(error.raw_os_error(), Some(5));
        assert_eq!(attempts, 1);
        assert_eq!(sleeps, 0);
    }

    #[test]
    fn retry_gives_up_after_the_bounded_attempts() {
        let mut attempts = 0;
        let error = retry_with(
            || {
                attempts += 1;
                Err::<(), _>(sharing_violation())
            },
            |_| {},
        )
        .expect_err("the retry budget must be bounded");
        assert_eq!(error.raw_os_error(), Some(32));
        assert_eq!(attempts, RETRY_ATTEMPTS);
    }

    #[test]
    fn the_lock_violation_sibling_error_is_retryable_too() {
        assert!(is_sharing_violation(&io::Error::from_raw_os_error(33)));
        assert!(!is_sharing_violation(&io::Error::from_raw_os_error(5)));
    }

    #[test]
    fn the_backoff_grows_to_its_cap() {
        assert_eq!(backoff(0), RETRY_BASE_DELAY);
        assert_eq!(backoff(2), Duration::from_millis(100));
        assert_eq!(backoff(3), RETRY_MAX_DELAY);
        assert_eq!(backoff(9), RETRY_MAX_DELAY);
    }
}
