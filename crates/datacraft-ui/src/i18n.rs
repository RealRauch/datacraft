//! UI localization kernel (step 18.1, item I1; ADR-4, ARCHITECTURE §15).
//!
//! Gettext-style `t()`/`tr()`: the English source string is the lookup key,
//! the embedded German catalog resolves it, and an unknown key falls back to
//! the English source unchanged so incremental coverage never breaks a build.
//! Language selection lives in the UI layer only — core, the sidecar and the
//! wire stay locale-neutral.

use std::sync::atomic::{AtomicU8, Ordering};

mod de;

/// UI display language (never leaves this crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English — the source language of every key.
    English,
    /// German.
    German,
}

impl Lang {
    /// Maps a system locale onto a language (`de*` → German, else English).
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(locale) if locale.trim().to_lowercase().starts_with("de") => Self::German,
            _ => Self::English,
        }
    }

    /// Maps the `DATACRAFT_LANGUAGE` override; unknown values yield `None`
    /// so the caller falls through to system detection (ADR-5).
    pub fn from_setting(value: &str) -> Option<Self> {
        let value = value.trim().to_lowercase();
        if value.is_empty() || value == "auto" {
            return None;
        }
        match value.split(['-', '_']).next() {
            Some("de") => Some(Self::German),
            Some("en") => Some(Self::English),
            _ => None,
        }
    }
}

/// Resolves the UI language: explicit setting > `DATACRAFT_LANGUAGE` >
/// system detection > English. Phase 19 passes the stored setting through
/// `env`; until then the environment and detection decide.
pub fn resolve_language(env: Option<&str>, system: Option<&str>) -> Lang {
    env.and_then(Lang::from_setting)
        .unwrap_or_else(|| Lang::from_locale(system))
}

const CURRENT_UNSET: u8 = 0;
const CURRENT_ENGLISH: u8 = 1;
const CURRENT_GERMAN: u8 = 2;

/// Process-wide current language, resolved once per session.
static CURRENT: AtomicU8 = AtomicU8::new(CURRENT_UNSET);

/// The current UI language (system detection and the environment override
/// are read once, then cached).
pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        CURRENT_ENGLISH => Lang::English,
        CURRENT_GERMAN => Lang::German,
        _ => {
            let resolved = resolve_language(
                std::env::var("DATACRAFT_LANGUAGE").ok().as_deref(),
                sys_locale::get_locale().as_deref(),
            );
            set_lang(resolved);
            resolved
        }
    }
}

/// Switches the current UI language (the language switcher of step 19.2
/// and the tests use this; the kernel itself only reads `lang()`).
pub fn set_lang(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::English => CURRENT_ENGLISH,
            Lang::German => CURRENT_GERMAN,
        },
        Ordering::Relaxed,
    );
}

/// Looks a key up in the given language (pure — no global state).
pub fn translate(lang: Lang, key: &'static str) -> &'static str {
    match lang {
        Lang::English => key,
        Lang::German => de::lookup(key).unwrap_or(key),
    }
}

/// The English source string rendered in the current UI language.
pub fn t(key: &'static str) -> &'static str {
    translate(lang(), key)
}

/// A template key with `{name}` placeholders rendered in the current
/// language; unknown placeholders stay visible instead of vanishing.
pub fn tr(key: &'static str, args: &[(&str, &str)]) -> String {
    fill(translate(lang(), key), args)
}

/// Substitutes `{name}` placeholders (pure).
fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// Count-bearing text with English/German singular and plural keys, e.g.
/// `plural(n, "{count} row", "{count} rows")`.
pub fn plural(count: usize, singular: &'static str, plural: &'static str) -> String {
    plural_in(lang(), count, singular, plural)
}

fn plural_in(lang: Lang, count: usize, singular: &'static str, plural: &'static str) -> String {
    let key = if count == 1 { singular } else { plural };
    let count_text = count.to_string();
    fill(translate(lang, key), &[("count", &count_text)])
}

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
