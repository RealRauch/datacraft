//! The recent-databases part of `ui.json` (items U15, M41) — split below
//! the file-size gate (step 92.2, item M108's L30 rule).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Settings;

/// Maximum number of recent database paths kept in `ui.json` (item U15).
pub const MAX_RECENT_DATABASES: usize = 10;

/// One recent-database entry (step 38.2, item M41): the path plus the file's
/// modification time (unix seconds) captured at record/load time, so the
/// backstage renders a "Modified" stamp without per-frame fs access.
#[derive(Debug, Clone, PartialEq)]
pub struct RecentDatabase {
    /// The database path.
    pub path: PathBuf,
    /// Modification time in unix seconds; `None` when the file could not be
    /// stat'ed.
    pub modified: Option<u64>,
}

impl Settings {
    /// Records a successfully opened database (item U15): newest first,
    /// de-duplicated case-insensitively on Windows, bounded. The file's
    /// modification time is captured here (step 38.2, item M41).
    pub fn record_recent(&mut self, path: &Path) {
        self.recent
            .retain(|candidate| !same_path(&candidate.path, path));
        self.recent.insert(
            0,
            RecentDatabase {
                path: path.to_path_buf(),
                modified: file_modified(path),
            },
        );
        self.recent.truncate(MAX_RECENT_DATABASES);
    }

    /// The recent-database entries, newest first.
    pub fn recent(&self) -> &[RecentDatabase] {
        &self.recent
    }
}

/// Localized startup notice for recent paths that no longer exist.
pub fn prune_notice(pruned: &[PathBuf]) -> String {
    if let [path] = pruned {
        return crate::i18n::tr(
            "A recent database was removed because it no longer exists: {path}",
            &[("path", &path.display().to_string())],
        );
    }
    let paths = pruned
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    crate::i18n::tr(
        "{count} recent databases were removed because they no longer exist: {paths}",
        &[("count", &pruned.len().to_string()), ("paths", &paths)],
    )
}

/// On-disk shape of one recent entry: the structured object since step 38.2
/// (item M41) or a legacy plain path string, which still loads.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum RecentFile {
    /// Structured entry.
    Entry {
        path: PathBuf,
        #[serde(default)]
        modified: Option<u64>,
    },
    /// Pre-38.2 plain path string.
    Legacy(PathBuf),
}

impl RecentFile {
    /// The entry's path regardless of the on-disk shape.
    pub(super) fn path(self) -> PathBuf {
        match self {
            RecentFile::Entry { path, .. } | RecentFile::Legacy(path) => path,
        }
    }
}

/// Path equality: Windows compares case-insensitively.
fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

/// The file's modification time in unix seconds; `None` when it cannot be
/// stat'ed (step 38.2, item M41).
pub(super) fn file_modified(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
}
