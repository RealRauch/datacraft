//! UI-owned app settings (step 19.1, item I4; ADR-5, ARCHITECTURE §15).
//!
//! `ui.json` lives in the platform config directory and holds UI preferences
//! only — never database design data. The `.datacraft` sidecar and every
//! core/wire contract stay untouched. Unknown or corrupt values fall back to
//! defaults instead of failing (WordCraft `ui.json` pattern).

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "datacraft-settings-{}-{name}",
            std::process::id()
        ));
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
