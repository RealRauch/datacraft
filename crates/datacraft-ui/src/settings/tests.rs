use super::*;
use crate::window::WindowGeometry;
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
        appearance: Appearance::Light,
        window: Some(WindowGeometry {
            width: 1280.0,
            height: 800.0,
            x: Some(40.0),
            y: Some(60.0),
            maximized: true,
        }),
        ..Settings::default()
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
        Some(WindowGeometry {
            width: 1024.0,
            height: 700.0,
            x: None,
            y: None,
            maximized: false,
        })
    );
}

#[test]
fn appearance_round_trips_and_unknown_values_fall_back() {
    let dir = temp_dir("appearance");
    let path = dir.join("ui.json");
    std::fs::write(&path, r#"{"appearance":"dark"}"#).expect("write");
    assert_eq!(Settings::load(&path).appearance, Appearance::Dark);
    std::fs::write(&path, r#"{"appearance":"neon"}"#).expect("write");
    assert_eq!(Settings::load(&path).appearance, Appearance::System);

    let settings = Settings {
        appearance: Appearance::Light,
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
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

    let home_only =
        |name: &str| -> Option<OsString> { (name == "HOME").then(|| OsString::from("/home/t")) };
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
    use crate::i18n::{Lang, lang_lock, set_lang};

    let _guard = lang_lock();
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

/// A crash between writing and renaming can only lose the new values —
/// never truncate the file that is already on disk (step 22.1, U13).
#[test]
fn a_failed_save_keeps_the_previous_file_intact() {
    let dir = temp_dir("atomic-failure");
    let path = dir.join("ui.json");
    let original = Settings {
        language: LanguageSetting::German,
        ..Settings::default()
    };
    original.save(&path).expect("save");
    let before = std::fs::read_to_string(&path).expect("read");

    // Occupy the sibling temp path so the write half cannot complete.
    let temp = temp_path(&path);
    std::fs::create_dir(&temp).expect("block temp path");
    let changed = Settings {
        language: LanguageSetting::English,
        ..Settings::default()
    };
    assert!(changed.save(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), before);
    std::fs::remove_dir(&temp).expect("remove blocker");
}

/// A stale temp file from an interrupted run never blocks the next save.
#[test]
fn a_stale_temp_file_is_replaced_on_the_next_save() {
    let dir = temp_dir("stale-temp");
    let path = dir.join("ui.json");
    Settings::default().save(&path).expect("save");
    std::fs::write(temp_path(&path), "{ truncated").expect("stale temp");

    let again = Settings {
        language: LanguageSetting::English,
        ..Settings::default()
    };
    again.save(&path).expect("save over stale temp");
    assert_eq!(Settings::load(&path), again);
    assert!(!temp_path(&path).exists(), "temp file must be renamed away");
}

#[test]
fn recent_databases_are_bounded_deduplicated_and_newest_first() {
    let dir = temp_dir("recents");
    let path = dir.join("ui.json");
    let mut settings = Settings::default();
    for index in 0..(MAX_RECENT_DATABASES + 2) {
        let candidate = dir.join(format!("db{index}.mdb"));
        std::fs::write(&candidate, b"x").expect("seed the recent path");
        settings.record_recent(&candidate);
    }
    assert_eq!(settings.recent.len(), MAX_RECENT_DATABASES);
    assert_eq!(
        settings.recent[0],
        dir.join(format!("db{}.mdb", MAX_RECENT_DATABASES + 1))
    );

    std::fs::write(dir.join("db5.mdb"), b"x").expect("seed the reopened path");
    settings.record_recent(&dir.join("db5.mdb"));
    assert_eq!(settings.recent[0], dir.join("db5.mdb"));
    assert_eq!(
        settings
            .recent
            .iter()
            .filter(|candidate| **candidate == dir.join("db5.mdb"))
            .count(),
        1,
        "a reopened path moves to the front instead of repeating"
    );

    settings.save(&path).expect("save");
    let loaded = Settings::load(&path);
    assert_eq!(loaded.recent, settings.recent, "the list survives restarts");
}

#[test]
fn missing_recent_paths_are_pruned_with_a_report() {
    let dir = temp_dir("prune");
    let path = dir.join("ui.json");
    let existing = dir.join("exists.mdb");
    std::fs::write(&existing, b"x").expect("seed the existing path");
    let missing = dir.join("missing.mdb");
    let settings = Settings {
        recent: vec![existing.clone(), missing.clone()],
        ..Settings::default()
    };
    settings.save(&path).expect("save");

    let loaded = Settings::load(&path);
    assert_eq!(loaded.recent, vec![existing], "stale paths are pruned");
    assert_eq!(
        loaded.pruned_recent,
        vec![missing],
        "the pruned path stays visible for the notice"
    );
}
