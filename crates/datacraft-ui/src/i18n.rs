//! UI localization kernel (step 18.1, item I1; ADR-4, ARCHITECTURE §15).
//!
//! Gettext-style `t()`/`tr()`: the English source string is the lookup key,
//! the embedded German catalog resolves it, and an unknown key falls back to
//! the English source unchanged so incremental coverage never breaks a build.
//! Language selection lives in the UI layer only — core, the sidecar and the
//! wire stay locale-neutral.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes the tests that touch the process-wide current language.
    static LANG_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn unknown_keys_fall_back_to_the_english_source() {
        assert_eq!(
            translate(Lang::German, "untranslated key"),
            "untranslated key"
        );
    }

    #[test]
    fn english_returns_the_source_even_when_the_catalog_has_an_entry() {
        assert_eq!(translate(Lang::English, "File"), "File");
    }

    #[test]
    fn german_resolves_catalog_entries() {
        assert_eq!(translate(Lang::German, "File"), "Datei");
    }

    #[test]
    fn locale_strings_map_to_languages() {
        for locale in ["de", "de-DE", "de_AT", "DE", "de-CH"] {
            assert_eq!(Lang::from_locale(Some(locale)), Lang::German, "{locale}");
        }
        for locale in [None, Some("en-US"), Some("fr"), Some("")] {
            assert_eq!(Lang::from_locale(locale), Lang::English, "{locale:?}");
        }
    }

    #[test]
    fn settings_accept_language_codes_and_ignore_unknown_values() {
        assert_eq!(Lang::from_setting("de"), Some(Lang::German));
        assert_eq!(Lang::from_setting(" DE-de "), Some(Lang::German));
        assert_eq!(Lang::from_setting("en"), Some(Lang::English));
        assert_eq!(Lang::from_setting("en-GB"), Some(Lang::English));
        assert_eq!(Lang::from_setting("auto"), None);
        assert_eq!(Lang::from_setting(""), None);
        assert_eq!(Lang::from_setting("fr"), None);
    }

    #[test]
    fn the_environment_override_beats_system_detection() {
        assert_eq!(resolve_language(Some("de"), Some("en-US")), Lang::German);
        assert_eq!(resolve_language(Some("en"), Some("de-DE")), Lang::English);
    }

    #[test]
    fn unknown_or_missing_overrides_fall_through_to_detection() {
        assert_eq!(resolve_language(Some("fr"), Some("de-DE")), Lang::German);
        assert_eq!(resolve_language(None, Some("de-DE")), Lang::German);
        assert_eq!(resolve_language(None, None), Lang::English);
    }

    #[test]
    fn templates_substitute_named_placeholders() {
        assert_eq!(
            fill("Design saved to {path}", &[("path", "/tmp/shop.datacraft")]),
            "Design saved to /tmp/shop.datacraft"
        );
        assert_eq!(fill("no placeholders", &[]), "no placeholders");
        assert_eq!(fill("{a} and {a}", &[("a", "x")]), "x and x");
    }

    #[test]
    fn plurals_pick_the_matching_key() {
        assert_eq!(
            plural_in(Lang::English, 1, "{count} row", "{count} rows"),
            "1 row"
        );
        assert_eq!(
            plural_in(Lang::English, 3, "{count} row", "{count} rows"),
            "3 rows"
        );
        assert_eq!(
            plural_in(Lang::German, 1, "{count} row", "{count} rows"),
            "1 Zeile"
        );
    }

    #[test]
    fn set_language_switches_the_current_language() {
        let _guard = LANG_LOCK.lock().expect("language lock");
        set_lang(Lang::German);
        assert_eq!(t("File"), "Datei");
        set_lang(Lang::English);
        assert_eq!(t("File"), "File");
    }
}
