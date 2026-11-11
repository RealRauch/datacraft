//! UI-owned app settings (step 19.1, item I4; ADR-5, ARCHITECTURE §15).
//!
//! `ui.json` lives in the platform config directory and holds UI preferences
//! only — never database design data. The `.datacraft` sidecar and every
//! core/wire contract stay untouched. Unknown or corrupt values fall back to
//! defaults instead of failing (WordCraft `ui.json` pattern).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::i18n::Lang;
use crate::window::WindowGeometry;

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

/// Persisted UI preferences: language, appearance, window geometry and the
/// bounded recent-database list (item U15).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// Language choice; `Auto` follows environment and system detection.
    pub language: LanguageSetting,
    /// Light/Dark/System choice; `System` follows the OS.
    pub appearance: Appearance,
    /// Last window geometry (sanitized; `None` uses the defaults).
    pub window: Option<WindowGeometry>,
    /// Recently opened databases, newest first, bounded by
    /// [`MAX_RECENT_DATABASES`] and de-duplicated (step 38.2, item M41:
    /// entries carry the file's modification time).
    pub recent: Vec<RecentDatabase>,
    /// Paths dropped on load because they no longer exist (not persisted;
    /// surfaced as a startup notice).
    pub pruned_recent: Vec<PathBuf>,
}

/// Language choice stored in `ui.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguageSetting {
    /// Follow `DATACRAFT_LANGUAGE` and system detection.
    #[default]
    Auto,
    /// German.
    German,
    /// English.
    English,
}

impl LanguageSetting {
    /// Maps the stored value; unknown values fall back to `Auto`.
    pub fn from_value(value: Option<&str>) -> Self {
        match value.and_then(Lang::from_setting) {
            Some(Lang::German) => Self::German,
            Some(Lang::English) => Self::English,
            _ => Self::Auto,
        }
    }

    /// Stored value of this choice.
    pub fn as_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::German => "de",
            Self::English => "en",
        }
    }

    /// Explicit language of this choice; `None` means detection (`Auto`).
    pub fn lang(self) -> Option<Lang> {
        match self {
            Self::Auto => None,
            Self::German => Some(Lang::German),
            Self::English => Some(Lang::English),
        }
    }

    /// Menu label: the endonyms stay fixed so every user can find their
    /// language; only `Auto` is translated.
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => crate::i18n::t("Auto"),
            Self::German => "Deutsch",
            Self::English => "English",
        }
    }
}

impl Settings {
    /// Loads the settings; a missing, empty or corrupt file yields defaults.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(file) = serde_json::from_str::<SettingsFile>(&text) else {
            return Self::default();
        };
        let mut settings = Self::from_file(file);
        for entry in &mut settings.recent {
            if entry.modified.is_none() {
                entry.modified = file_modified(&entry.path);
            }
        }
        let mut pruned = Vec::new();
        settings.recent.retain(|entry| {
            if entry.path.exists() {
                true
            } else {
                pruned.push(entry.path.clone());
                false
            }
        });
        settings.pruned_recent = pruned;
        settings
    }

    /// Writes the settings, creating the config directory as needed. The
    /// JSON goes to a sibling temporary file that is renamed over the
    /// target, so an interrupted write can never truncate the file that is
    /// already on disk (step 22.1, U13).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let file = SettingsFile {
            language: Some(self.language.as_value().to_owned()),
            appearance: Some(self.appearance.as_value().to_owned()),
            window: self.window,
            recent: self
                .recent
                .iter()
                .map(|entry| RecentFile::Entry {
                    path: entry.path.clone(),
                    modified: entry.modified,
                })
                .collect(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = temp_path(path);
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, path)
    }

    /// Records the current viewport geometry (implausible parts are
    /// dropped; the sanitizer lives in `window.rs`).
    pub fn set_window(
        &mut self,
        width: f32,
        height: f32,
        position: Option<(f32, f32)>,
        maximized: bool,
    ) {
        let geometry = WindowGeometry {
            width,
            height,
            x: position.map(|(x, _)| x),
            y: position.map(|(_, y)| y),
            maximized,
        };
        self.window = geometry.sane();
    }

    fn from_file(file: SettingsFile) -> Self {
        Self {
            language: LanguageSetting::from_value(file.language.as_deref()),
            appearance: Appearance::from_value(file.appearance.as_deref()),
            window: file.window.and_then(WindowGeometry::sane),
            recent: file
                .recent
                .into_iter()
                .map(|entry| RecentDatabase {
                    path: entry.path(),
                    modified: None,
                })
                .filter(|entry| !entry.path.as_os_str().is_empty())
                .collect(),
            pruned_recent: Vec::new(),
        }
    }

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
fn file_modified(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
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

/// Sibling temporary path of `path` (`ui.json` → `ui.json.tmp`).
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// On-disk shape: strings and optional fields so one unknown value cannot
/// discard the rest of the file.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct SettingsFile {
    language: Option<String>,
    appearance: Option<String>,
    window: Option<WindowGeometry>,
    #[serde(default)]
    recent: Vec<RecentFile>,
}

/// On-disk shape of one recent entry: the structured object since step 38.2
/// (item M41) or a legacy plain path string, which still loads.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum RecentFile {
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
    fn path(self) -> PathBuf {
        match self {
            RecentFile::Entry { path, .. } | RecentFile::Legacy(path) => path,
        }
    }
}

/// Platform conventions for the config directory (resolved without a new
/// dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    MacOs,
    Linux,
}

fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Linux
    }
}

fn config_dir_on(platform: Platform, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
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

fn default_path_on(platform: Platform, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    config_dir_on(platform, env).map(|dir| dir.join("datacraft").join("ui.json"))
}

/// `<config>/datacraft/ui.json` for the running platform; `None` when no
/// config directory can be resolved (settings then stay session-only).
pub fn default_path() -> Option<PathBuf> {
    default_path_on(platform(), &|name| std::env::var_os(name))
}

impl crate::app::App {
    /// Attaches the UI settings file (the composition root sets this).
    pub fn set_settings_path(&mut self, path: Option<PathBuf>) {
        self.settings_path = path;
    }

    /// Attaches the dialog host: the binary uses the native dialogs, the
    /// headless suite injects scripted answers (step 38.3, item M43).
    pub fn set_dialogs(&mut self, host: Box<dyn crate::dialogs::DialogHost>) {
        self.dialogs = host;
    }

    /// Applies persisted settings at startup: the stored language choice
    /// beats the environment and system detection (ADR-5).
    pub fn apply_settings(&mut self, settings: Settings) {
        if !settings.pruned_recent.is_empty() {
            self.record_notice(prune_notice(&settings.pruned_recent));
        }
        self.settings = settings;
        crate::i18n::init_language(self.language_setting().lang());
    }

    /// Recent database entries, newest first (item U15).
    pub fn recent_databases(&self) -> &[RecentDatabase] {
        self.settings.recent()
    }

    /// Current language choice.
    pub fn language_setting(&self) -> LanguageSetting {
        self.settings.language
    }

    /// Current appearance choice.
    pub fn appearance_setting(&self) -> Appearance {
        self.settings.appearance
    }

    /// Switches the appearance and persists the choice (ADR-6). The next
    /// frame applies it as egui's theme preference.
    pub fn set_appearance_setting(&mut self, setting: Appearance) {
        self.settings.appearance = setting;
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Switches the UI language live and persists the choice (ADR-5).
    pub fn set_language_setting(&mut self, setting: LanguageSetting) {
        self.settings.language = setting;
        crate::i18n::init_language(setting.lang());
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Records a successful open in the recent list and persists it
    /// (step 28.2, item U15).
    pub(crate) fn record_recent_database(&mut self, path: &Path) {
        self.settings.record_recent(path);
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Writes the settings file; `Ok(())` when none is attached.
    pub fn persist_settings(&self) -> std::io::Result<()> {
        match &self.settings_path {
            Some(path) => self.settings.save(path),
            None => Ok(()),
        }
    }

    /// Records the current viewport geometry for the next save.
    pub(crate) fn capture_window_geometry(&mut self, ctx: &eframe::egui::Context) {
        let viewport = ctx.input(|input| input.viewport().clone());
        let Some(size) = viewport.inner_rect.map(|rect| rect.size()) else {
            return;
        };
        let position = viewport.outer_rect.map(|rect| (rect.min.x, rect.min.y));
        let maximized = viewport.maximized.unwrap_or(false);
        self.settings
            .set_window(size.x, size.y, position, maximized);
    }

    /// Moves a window restored off-screen back onto the visible monitor,
    /// once per session (step 22.2, U13).
    pub(crate) fn rescue_window_position(&mut self, ctx: &eframe::egui::Context) {
        if self.position_rescued {
            return;
        }
        let Some(monitor) = ctx.input(|input| input.viewport().monitor_size) else {
            return;
        };
        self.position_rescued = true;
        let Some(geometry) = self.settings.window else {
            return;
        };
        let (Some(x), Some(y)) = (geometry.x, geometry.y) else {
            return;
        };
        let saved = eframe::egui::pos2(x, y);
        let size = eframe::egui::vec2(geometry.width, geometry.height);
        let target = crate::window::rescued_position(saved, size, monitor);
        if target != saved {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(target));
        }
    }
}

#[cfg(test)]
mod tests;
