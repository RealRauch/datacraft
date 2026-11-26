use super::platform::{Platform, config_dir_on, default_path_on};
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
        settings.recent[0].path,
        dir.join(format!("db{}.mdb", MAX_RECENT_DATABASES + 1))
    );

    std::fs::write(dir.join("db5.mdb"), b"x").expect("seed the reopened path");
    settings.record_recent(&dir.join("db5.mdb"));
    assert_eq!(settings.recent[0].path, dir.join("db5.mdb"));
    assert_eq!(
        settings
            .recent
            .iter()
            .filter(|candidate| candidate.path == dir.join("db5.mdb"))
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
        recent: vec![
            RecentDatabase {
                path: existing.clone(),
                modified: None,
            },
            RecentDatabase {
                path: missing.clone(),
                modified: None,
            },
        ],
        ..Settings::default()
    };
    settings.save(&path).expect("save");

    let loaded = Settings::load(&path);
    assert_eq!(loaded.recent.len(), 1, "stale paths are pruned");
    assert_eq!(loaded.recent[0].path, existing);
    assert_eq!(
        loaded.pruned_recent,
        vec![missing],
        "the pruned path stays visible for the notice"
    );
}

/// Step 38.2 (item M41): a recorded recent entry carries the file's mtime so
/// the backstage can show a "Modified" timestamp without per-frame fs access.
#[test]
fn recent_entries_capture_the_file_mtime() {
    let dir = temp_dir("recent-mtime");
    let candidate = dir.join("stamp.mdb");
    std::fs::write(&candidate, b"x").expect("seed the file");
    let mut settings = Settings::default();
    settings.record_recent(&candidate);
    assert_eq!(settings.recent.len(), 1);
    assert_eq!(settings.recent[0].path, candidate);
    assert!(
        settings.recent[0].modified.is_some(),
        "the mtime is captured at record time"
    );
}

/// Step 38.2 (item M41): `ui.json` files written before the structured
/// entries (plain path strings) still load; the mtime is filled from the file.
#[test]
fn legacy_plain_string_recent_entries_still_load() {
    let dir = temp_dir("recent-legacy");
    let existing = dir.join("legacy.mdb");
    std::fs::write(&existing, b"x").expect("seed the file");
    let path = dir.join("ui.json");
    let json = serde_json::json!({
        "language": "en",
        "recent": [existing.to_string_lossy()],
    });
    std::fs::write(&path, json.to_string()).expect("write the legacy settings");

    let loaded = Settings::load(&path);
    assert_eq!(loaded.recent.len(), 1, "the legacy entry survives");
    assert_eq!(loaded.recent[0].path, existing);
    assert!(
        loaded.recent[0].modified.is_some(),
        "the mtime is filled from the file at load"
    );
}

/// Step 38.2 (item M41): the backstage renders the captured mtime as a
/// deterministic ISO date (no new dependency, no locale drift).
#[test]
fn recent_modified_renders_as_an_iso_date() {
    use crate::backstage::recent::iso_date_from_unix;
    assert_eq!(iso_date_from_unix(0), "1970-01-01");
    assert_eq!(iso_date_from_unix(1_793_664_000), "2026-11-03");
}

/// Per-surface pane geometry round-trips through `ui.json` (step 92.2,
/// item M108).
#[test]
fn pane_geometry_round_trips_through_the_settings_file() {
    let dir = temp_dir("panes");
    let path = dir.join("ui.json");
    let settings = Settings {
        panes: std::iter::once((
            "sql".to_owned(),
            crate::panes::PaneGeometry {
                mode: crate::panes::DockMode::Right,
                size: 420.0,
            },
        ))
        .collect(),
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
}

/// Broken pane entries fall back: an unknown mode floats, a docked size
/// clamps into its bounds, a missing size takes the mode default
/// (step 92.2, item M108).
#[test]
fn unknown_pane_entries_fall_back_to_sane_values() {
    let dir = temp_dir("panes-broken");
    let path = dir.join("ui.json");
    std::fs::write(
        &path,
        r#"{"panes":{"sql":{"mode":"sideways"},"x":{"mode":"right","size":9000.0}}}"#,
    )
    .expect("write");
    let loaded = Settings::load(&path);
    assert_eq!(loaded.panes["sql"].mode, crate::panes::DockMode::Float);
    assert_eq!(
        loaded.panes["x"].mode,
        crate::panes::DockMode::Right,
        "the pane key itself is opaque — values stay"
    );
    assert!(
        loaded.panes["x"].size <= crate::panes::PaneGeometry::RIGHT_MAX,
        "{:?}",
        loaded.panes["x"]
    );
}

/// Nav preferences round-trip per database path (step 92.5, item L48).
#[test]
fn nav_preferences_round_trip_per_database() {
    let dir = temp_dir("nav");
    let path = dir.join("ui.json");
    let settings = Settings {
        nav: std::iter::once((
            "shop.mdb".to_owned(),
            crate::nav_groups::NavPrefs {
                view: crate::nav_groups::NavView::Details,
                groups: vec![crate::nav_groups::NavGroup {
                    name: "Mine".to_owned(),
                    colour: crate::nav_groups::NavColour::Teal,
                    objects: vec![datacraft_core::ObjectId::new(
                        datacraft_core::ObjectKind::Table,
                        "Customers",
                    )],
                }],
            },
        ))
        .collect(),
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
}

/// Broken nav entries fall back: an unknown view stays in the tree, an
/// unknown colour takes the default, an unnamed group is skipped
/// (step 92.5, item L48).
#[test]
fn broken_nav_entries_fall_back_and_unnamed_groups_are_skipped() {
    let dir = temp_dir("nav-broken");
    let path = dir.join("ui.json");
    std::fs::write(
        &path,
        r#"{"nav":{"a.mdb":{"view":"sideways","groups":[{"colour":"pink"}],"x":1},"b.mdb":{"groups":[{"name":"G","colour":"teal","objects":[{"kind":"table","name":"T"}]}]}}}"#,
    )
    .expect("write");
    let loaded = Settings::load(&path);
    assert_eq!(loaded.nav["a.mdb"].view, crate::nav_groups::NavView::Tree);
    assert!(loaded.nav["a.mdb"].groups.is_empty());
    let group = &loaded.nav["b.mdb"].groups[0];
    assert_eq!(group.name, "G");
    assert_eq!(group.colour, crate::nav_groups::NavColour::Teal);
    assert_eq!(group.objects.len(), 1);
}

/// The low-power setting round-trips (step 92.6, item L51) — `None`
/// means auto-detection.
#[test]
fn the_low_power_setting_round_trips() {
    let dir = temp_dir("low-power");
    let path = dir.join("ui.json");
    let settings = Settings {
        low_power: Some(true),
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
    let settings = Settings {
        low_power: Some(false),
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
    assert_eq!(Settings::default().low_power, None);
}

#[test]
fn the_preview_zoom_round_trips_and_rejects_implausible_values() {
    let dir = temp_dir("preview-zoom");
    let path = dir.join("ui.json");
    let settings = Settings {
        preview_zoom: 1.5,
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path).preview_zoom, 1.5);

    // Implausible stored values fall back to the default zoom.
    for stored in ["0.0", "-1.0", "99.0"] {
        std::fs::write(&path, format!(r#"{{"preview_zoom":{stored}}}"#)).expect("write");
        assert_eq!(
            Settings::load(&path).preview_zoom,
            sane_preview_zoom(None),
            "stored {stored}"
        );
    }
    // The default keeps a page inside typical window heights.
    assert!((sane_preview_zoom(None) - 0.9).abs() < f32::EPSILON);
    assert!((sane_preview_zoom(Some(1.25)) - 1.25).abs() < f32::EPSILON);
    assert!((sane_preview_zoom(Some(f32::NAN)) - 0.9).abs() < f32::EPSILON);
}
