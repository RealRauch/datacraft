//! App settings and window-title tests (steps 19.1/21.7; split from
//! `main.rs` to keep every file below the size gate).
//!
//! These exercise the UI-owned `ui.json` store, the language/appearance
//! persistence and the Access-style viewport title through the public
//! app surface.

use crate::app::App;

#[test]
fn startup_settings_apply_the_language_and_survive_a_restart() {
    use crate::settings::{LanguageSetting, Settings};

    let _guard = crate::i18n::lang_lock();
    let dir =
        std::env::temp_dir().join(format!("datacraft-startup-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    Settings {
        language: LanguageSetting::German,
        ..Settings::default()
    }
    .save(&path)
    .expect("save");

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_settings_path(Some(path.clone()));
    app.apply_settings(Settings::load(&path));
    assert_eq!(app.language_setting(), LanguageSetting::German);
    assert_eq!(crate::i18n::t("File"), "Datei");

    crate::i18n::set_lang(crate::i18n::Lang::English);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn switching_the_language_applies_live_and_persists_the_choice() {
    use crate::settings::LanguageSetting;

    let _guard = crate::i18n::lang_lock();
    let dir =
        std::env::temp_dir().join(format!("datacraft-language-switch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ui.json");

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_settings_path(Some(path.clone()));
    app.set_language_setting(LanguageSetting::German);
    assert_eq!(app.language_setting(), LanguageSetting::German);
    assert_eq!(crate::i18n::t("File"), "Datei");
    assert!(app.last_error().is_none(), "{:?}", app.last_error());

    // The next start reads the same choice (acceptance: restart).
    let loaded = crate::settings::Settings::load(&path);
    let mut restarted = App::new(Box::new(datacraft_core::port::MockPort::new()));
    restarted.apply_settings(loaded);
    assert_eq!(restarted.language_setting(), LanguageSetting::German);
    assert_eq!(crate::i18n::t("File"), "Datei");

    restarted.set_language_setting(LanguageSetting::English);
    assert_eq!(crate::i18n::t("File"), "File");
    crate::i18n::set_lang(crate::i18n::Lang::English);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_settings_write_surfaces_a_precise_reason() {
    use crate::settings::LanguageSetting;

    let _guard = crate::i18n::lang_lock();
    crate::i18n::set_lang(crate::i18n::Lang::English);
    let dir = std::env::temp_dir().join(format!("datacraft-settings-fail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    // A directory path cannot be written as a file. Keep the language
    // English so the surfaced message itself is deterministic.
    app.set_settings_path(Some(dir.clone()));
    app.set_language_setting(LanguageSetting::English);
    assert!(
        app.last_error()
            .unwrap_or_default()
            .contains("could not be saved"),
        "{:?}",
        app.last_error()
    );

    crate::i18n::set_lang(crate::i18n::Lang::English);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn window_title_follows_the_open_database_and_clears_on_close() {
    let _guard = crate::i18n::lang_lock();
    crate::i18n::set_lang(crate::i18n::Lang::English);
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    assert_eq!(crate::shell::window_title(app.snapshot()), "DataCraft");
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    assert_eq!(
        crate::shell::window_title(app.snapshot()),
        "shop : Database (Jet4)"
    );
    app.dispatch(datacraft_core::command::Command::CloseDatabase);
    assert_eq!(crate::shell::window_title(app.snapshot()), "DataCraft");
}

#[test]
fn window_title_is_localized_in_both_languages() {
    let _guard = crate::i18n::lang_lock();
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    crate::i18n::set_lang(crate::i18n::Lang::German);
    assert_eq!(
        crate::shell::window_title(app.snapshot()),
        "shop : Datenbank (Jet4)"
    );
    crate::i18n::set_lang(crate::i18n::Lang::English);
    assert_eq!(
        crate::shell::window_title(app.snapshot()),
        "shop : Database (Jet4)"
    );
}
