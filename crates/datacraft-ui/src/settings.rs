//! UI-owned app settings (step 19.1, item I4; ADR-5, ARCHITECTURE §15).
//!
//! `ui.json` lives in the platform config directory and holds UI preferences
//! only — never database design data. The `.datacraft` sidecar and every
//! core/wire contract stay untouched. Unknown or corrupt values fall back to
//! defaults instead of failing (WordCraft `ui.json` pattern).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

/// Persisted UI preferences: language and window geometry now, appearance
/// with step U4.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// Language choice; `Auto` follows environment and system detection.
    pub language: LanguageSetting,
    /// Last inner window size (sanitized; `None` uses the default).
    pub window: Option<WindowSize>,
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
}

/// Window size in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    /// Inner width.
    pub width: f32,
    /// Inner height.
    pub height: f32,
}

impl WindowSize {
    /// Drops implausible geometry (zero, negative, NaN, absurd) so a corrupt
    /// file never yields an unusable window.
    fn sane(self) -> Option<Self> {
        let usable = self.width.is_finite()
            && self.height.is_finite()
            && (320.0..=16_384.0).contains(&self.width)
            && (240.0..=16_384.0).contains(&self.height);
        usable.then_some(self)
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
        Self::from_file(file)
    }

    /// Writes the settings, creating the config directory as needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let file = SettingsFile {
            language: Some(self.language.as_value().to_owned()),
            window: self.window,
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    /// Records the current viewport size (ignored when implausible).
    pub fn set_window(&mut self, width: f32, height: f32) {
        self.window = WindowSize { width, height }.sane();
    }

    fn from_file(file: SettingsFile) -> Self {
        Self {
            language: LanguageSetting::from_value(file.language.as_deref()),
            window: file.window.and_then(WindowSize::sane),
        }
    }
}

/// On-disk shape: strings and optional fields so one unknown value cannot
/// discard the rest of the file.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct SettingsFile {
    language: Option<String>,
    window: Option<WindowSize>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("datacraft-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn settings_round_trip_preserves_language_and_window() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("ui.json");
        let settings = Settings {
            language: LanguageSetting::German,
            window: Some(WindowSize {
                width: 1280.0,
                height: 800.0,
            }),
        };
        settings.save(&path).expect("save");
        assert_eq!(Settings::load(&path), settings);
    }

    #[test]
    fn corrupt_empty_or_missing_files_fall_back_to_defaults() {
        let dir = temp_dir("corrupt");
        let path = dir.join("ui.json");
        std::fs::write(&path, "{ not json").expect("write");
        assert_eq!(Settings::load(&path), Settings::default());
        std::fs::write(&path, "").expect("write");
        assert_eq!(Settings::load(&path), Settings::default());
        assert_eq!(
            Settings::load(&dir.join("missing.json")),
            Settings::default()
        );
    }

    #[test]
    fn unknown_language_values_keep_the_other_fields() {
        let dir = temp_dir("unknown-language");
        let path = dir.join("ui.json");
        std::fs::write(
            &path,
            r#"{"language":"fr","window":{"width":1024.0,"height":700.0}}"#,
        )
        .expect("write");
        let loaded = Settings::load(&path);
        assert_eq!(loaded.language, LanguageSetting::Auto);
        assert_eq!(
            loaded.window,
            Some(WindowSize {
                width: 1024.0,
                height: 700.0
            })
        );
    }

    #[test]
    fn unusable_geometry_is_dropped() {
        let dir = temp_dir("geometry");
        let path = dir.join("ui.json");
        std::fs::write(&path, r#"{"window":{"width":0.0,"height":700.0}}"#).expect("write");
        assert_eq!(Settings::load(&path).window, None);
        std::fs::write(&path, r#"{"window":{"width":1280.0,"height":-1.0}}"#).expect("write");
        assert_eq!(Settings::load(&path).window, None);
    }

    #[test]
    fn language_values_map_to_the_setting_and_back() {
        assert_eq!(LanguageSetting::from_value(None), LanguageSetting::Auto);
        assert_eq!(
            LanguageSetting::from_value(Some("auto")),
            LanguageSetting::Auto
        );
        assert_eq!(
            LanguageSetting::from_value(Some("de")),
            LanguageSetting::German
        );
        assert_eq!(
            LanguageSetting::from_value(Some("DE-de")),
            LanguageSetting::German
        );
        assert_eq!(
            LanguageSetting::from_value(Some("en")),
            LanguageSetting::English
        );
        assert_eq!(
            LanguageSetting::from_value(Some("fr")),
            LanguageSetting::Auto
        );
        for setting in [
            LanguageSetting::Auto,
            LanguageSetting::German,
            LanguageSetting::English,
        ] {
            assert_eq!(
                LanguageSetting::from_value(Some(setting.as_value())),
                setting
            );
        }
    }

    #[test]
    fn the_config_directory_follows_the_platform_conventions() {
        let env = |name: &str| -> Option<OsString> {
            match name {
                "APPDATA" => Some(OsString::from("C:\\Users\\t\\AppData\\Roaming")),
                "HOME" => Some(OsString::from("/home/t")),
                "XDG_CONFIG_HOME" => Some(OsString::from("/home/t/.config-custom")),
                _ => None,
            }
        };
        assert_eq!(
            config_dir_on(Platform::Windows, &env),
            Some(PathBuf::from("C:\\Users\\t\\AppData\\Roaming"))
        );
        assert_eq!(
            config_dir_on(Platform::Linux, &env),
            Some(PathBuf::from("/home/t/.config-custom"))
        );
        assert_eq!(
            config_dir_on(Platform::MacOs, &env),
            Some(PathBuf::from("/home/t/Library/Application Support"))
        );

        let home_only = |name: &str| -> Option<OsString> {
            (name == "HOME").then(|| OsString::from("/home/t"))
        };
        assert_eq!(
            config_dir_on(Platform::Linux, &home_only),
            Some(PathBuf::from("/home/t/.config"))
        );
        assert_eq!(config_dir_on(Platform::Windows, &home_only), None);
        assert_eq!(
            default_path_on(Platform::Linux, &home_only),
            Some(PathBuf::from("/home/t/.config/datacraft/ui.json"))
        );
    }

    #[test]
    fn language_labels_switch_with_the_current_language() {
        use crate::i18n::{LANG_LOCK, Lang, set_lang};

        let _guard = LANG_LOCK.lock().expect("language lock");
        set_lang(Lang::German);
        assert_eq!(LanguageSetting::Auto.label(), "Automatisch");
        assert_eq!(LanguageSetting::German.label(), "Deutsch");
        assert_eq!(LanguageSetting::English.label(), "English");
        set_lang(Lang::English);
        assert_eq!(LanguageSetting::Auto.label(), "Auto");
    }

    #[test]
    fn saving_writes_only_the_settings_file() {
        let dir = temp_dir("only-ui-json");
        let path = dir.join("nested").join("ui.json");
        Settings::default().save(&path).expect("save");
        let entries: Vec<String> = std::fs::read_dir(path.parent().expect("parent"))
            .expect("read dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(entries, vec!["ui.json".to_owned()]);
        assert!(!dir.join("shop.datacraft").exists());
    }
}
