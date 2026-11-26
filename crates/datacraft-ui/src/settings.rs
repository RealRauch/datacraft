//! UI-owned app settings (step 19.1, item I4; ADR-5, ARCHITECTURE §15).
//!
//! `ui.json` lives in the platform config directory and holds UI preferences
//! only — never database design data. The `.datacraft` sidecar and every
//! core/wire contract stay untouched. Unknown or corrupt values fall back to
//! defaults instead of failing (WordCraft `ui.json` pattern).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::i18n::Lang;
use crate::window::WindowGeometry;

mod app;
mod platform;
mod recent;

pub use recent::{MAX_RECENT_DATABASES, RecentDatabase, prune_notice};
use recent::{RecentFile, file_modified};

/// Default print-preview zoom: 0.9 keeps a page inside typical window
/// heights (phase 102, step 102.4).
pub const DEFAULT_PREVIEW_ZOOM: f32 = 0.9;

/// Accepted print-preview zoom range; anything else falls back.
const PREVIEW_ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;

/// A stored zoom, or the default when it is missing, non-finite or out of
/// range (the settings file's tolerance rule).
pub(crate) fn sane_preview_zoom(value: Option<f32>) -> f32 {
    match value {
        Some(zoom) if zoom.is_finite() && PREVIEW_ZOOM_RANGE.contains(&zoom) => zoom,
        _ => DEFAULT_PREVIEW_ZOOM,
    }
}

/// Persisted UI preferences: language, appearance, window geometry and the
/// bounded recent-database list (item U15).
#[derive(Debug, Clone, PartialEq)]
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
    /// Per-surface task-pane geometry keyed by an opaque surface id
    /// (`"sql"` today; step 92.2, item M108). Values are sane (clamped).
    pub panes: std::collections::BTreeMap<String, crate::panes::PaneGeometry>,
    /// Navigation-pane preferences keyed by the database path (view mode
    /// and local groups; step 92.5, item L48). Never in the sidecar.
    pub nav: std::collections::BTreeMap<String, crate::nav_groups::NavPrefs>,
    /// The low-power rendering choice (step 92.6, item L51): `None`
    /// follows the detection, `Some(true)`/`Some(false)` override it.
    pub low_power: Option<bool>,
    /// Print-preview zoom (phase 102, step 102.4) — a per-surface setting
    /// of the report preview, sanitized on load.
    pub preview_zoom: f32,
    /// Paths dropped on load because they no longer exist (not persisted;
    /// surfaced as a startup notice).
    pub pruned_recent: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: LanguageSetting::default(),
            appearance: Appearance::default(),
            window: None,
            recent: Vec::new(),
            panes: std::collections::BTreeMap::new(),
            nav: std::collections::BTreeMap::new(),
            low_power: None,
            preview_zoom: DEFAULT_PREVIEW_ZOOM,
            pruned_recent: Vec::new(),
        }
    }
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
            panes: self
                .panes
                .iter()
                .map(|(surface, geometry)| {
                    (
                        surface.clone(),
                        PaneFile {
                            mode: Some(geometry.mode.as_value().to_owned()),
                            size: Some(geometry.size),
                        },
                    )
                })
                .collect(),
            nav: self
                .nav
                .iter()
                .filter_map(|(key, prefs)| {
                    serde_json::to_value(prefs)
                        .ok()
                        .map(|value| (key.clone(), value))
                })
                .collect(),
            low_power: self.low_power,
            preview_zoom: Some(self.preview_zoom),
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
            panes: file
                .panes
                .into_iter()
                .map(|(surface, entry)| {
                    let mode = crate::panes::DockMode::from_value(entry.mode.as_deref());
                    (
                        surface,
                        crate::panes::PaneGeometry {
                            mode,
                            size: entry.size.unwrap_or_else(|| {
                                crate::panes::PaneGeometry::default_for(mode).size
                            }),
                        }
                        .sane(),
                    )
                })
                .collect(),
            nav: file
                .nav
                .into_iter()
                .filter_map(|(key, value)| {
                    // Per-entry tolerance: one broken database entry must
                    // not discard the others (the settings-file rule).
                    serde_json::from_value::<NavPrefsFile>(value)
                        .ok()
                        .map(|entry| (key, entry.into_prefs()))
                })
                .collect(),
            low_power: file.low_power,
            preview_zoom: sane_preview_zoom(file.preview_zoom),
            pruned_recent: Vec::new(),
        }
    }
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
    #[serde(default)]
    panes: std::collections::BTreeMap<String, PaneFile>,
    #[serde(default)]
    nav: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    low_power: Option<bool>,
    #[serde(default)]
    preview_zoom: Option<f32>,
}

/// Tolerant on-disk shape of one database's nav preferences (step 92.5,
/// item L48): unknown values fall back per field instead of dropping the
/// entry.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct NavPrefsFile {
    view: Option<String>,
    groups: Vec<NavGroupFile>,
}

impl NavPrefsFile {
    /// Maps the tolerated values onto the in-memory preferences.
    fn into_prefs(self) -> crate::nav_groups::NavPrefs {
        crate::nav_groups::NavPrefs {
            view: crate::nav_groups::NavView::from_value(self.view.as_deref()),
            groups: self
                .groups
                .into_iter()
                .filter_map(|group| {
                    let name = group.name?;
                    if name.trim().is_empty() {
                        return None;
                    }
                    Some(crate::nav_groups::NavGroup {
                        name,
                        colour: crate::nav_groups::NavColour::from_value(group.colour.as_deref()),
                        objects: group.objects,
                    })
                })
                .collect(),
        }
    }
}

/// Tolerant on-disk shape of one group.
#[derive(Debug, Deserialize)]
struct NavGroupFile {
    name: Option<String>,
    colour: Option<String>,
    #[serde(default)]
    objects: Vec<datacraft_core::ObjectId>,
}

/// On-disk shape of one pane geometry (step 92.2, item M108): optional
/// fields so one broken value cannot discard the entry's siblings.
#[derive(Debug, Serialize, Deserialize)]
struct PaneFile {
    mode: Option<String>,
    size: Option<f32>,
}

pub use platform::default_path;

#[cfg(test)]
mod tests;
