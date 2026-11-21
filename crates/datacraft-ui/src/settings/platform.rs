//! Platform conventions for the config directory (resolved without a
//! new dependency) — split from `settings.rs` for the file-size gate
//! (item L30).

use std::ffi::OsString;
use std::path::PathBuf;

/// The running platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Platform {
    Windows,
    MacOs,
    Linux,
}

pub(super) fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Linux
    }
}

pub(super) fn config_dir_on(
    platform: Platform,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    match platform {
        Platform::Windows => env("APPDATA").map(PathBuf::from),
        Platform::MacOs => env("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        }),
        Platform::Linux => env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config"))),
    }
}

pub(super) fn default_path_on(
    platform: Platform,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    config_dir_on(platform, env).map(|dir| dir.join("datacraft").join("ui.json"))
}

/// `<config>/datacraft/ui.json` for the running platform; `None` when no
/// config directory can be resolved (settings then stay session-only).
pub fn default_path() -> Option<PathBuf> {
    default_path_on(platform(), &|name| std::env::var_os(name))
}
