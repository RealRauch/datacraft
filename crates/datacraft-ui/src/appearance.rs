//! Appearance setting (step 20.4, item U4; ADR-6).
//!
//! `Appearance { Light, Dark, System }` (default `System`), persisted in
//! `ui.json` and applied as egui's theme preference. Both palettes are
//! installed by `theme::apply`; `theme::tokens` picks the set per frame from
//! `visuals.dark_mode`. On Linux, where winit reports no system theme, the
//! XDG desktop portal's `color-scheme` is polled (the model's
//! `desktop_theme.rs` pattern) — `parse_color_scheme` is the tested seam.

use eframe::egui;

use crate::i18n::t;

/// Light/Dark/System choice; unknown persisted values fall back to System.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    /// Follow the operating system.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Appearance {
    /// Maps the stored value; unknown values fall back to `System`.
    pub fn from_value(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    /// Stored value of this choice.
    pub fn as_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Menu label, localized via the UI catalog.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => t("System"),
            Self::Light => t("Light"),
            Self::Dark => t("Dark"),
        }
    }
}

/// The egui preference for a choice. `System` stays system-level unless the
/// platform reports its own value through the portal.
pub fn preference_for(
    appearance: Appearance,
    system: Option<egui::Theme>,
) -> egui::ThemePreference {
    match appearance {
        Appearance::Light => egui::ThemePreference::Light,
        Appearance::Dark => egui::ThemePreference::Dark,
        Appearance::System => system.map_or(egui::ThemePreference::System, Into::into),
    }
}

/// Applies the choice to the context (cheap enough to call every frame).
pub fn apply(ctx: &egui::Context, appearance: Appearance) {
    ctx.set_theme(preference_for(appearance, system_theme_override()));
}

fn system_theme_override() -> Option<egui::Theme> {
    #[cfg(target_os = "linux")]
    {
        portal::latest()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Parses the portal's `color-scheme` reply — a uint32 variant:
/// 0 = no preference, 1 = dark, 2 = light.
#[cfg(any(target_os = "linux", test))]
pub fn parse_color_scheme(output: &str) -> Option<egui::Theme> {
    let value = output.split("uint32").nth(1)?;
    let code: String = value
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    match code.as_str() {
        "1" => Some(egui::Theme::Dark),
        "2" => Some(egui::Theme::Light),
        _ => None,
    }
}

/// Linux live system-theme watch through the XDG desktop portal. Spawned
/// lazily on the first `System` application; polls every 4 s (`gdbus`).
#[cfg(target_os = "linux")]
mod portal {
    use std::sync::{Mutex, Once};
    use std::time::Duration;

    use eframe::egui::Theme;

    static LATEST: Mutex<Option<Theme>> = Mutex::new(None);
    static START: Once = Once::new();

    /// The latest reported theme; starts the poller on first use.
    pub(super) fn latest() -> Option<Theme> {
        START.call_once(|| {
            std::thread::spawn(poll);
        });
        LATEST.lock().ok().and_then(|latest| *latest)
    }

    fn poll() {
        let mut last = None;
        loop {
            let theme = read();
            if theme != last {
                if let Ok(mut latest) = LATEST.lock() {
                    *latest = theme;
                }
                last = theme;
            }
            std::thread::sleep(Duration::from_secs(4));
        }
    }

    fn read() -> Option<Theme> {
        let output = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.portal.Desktop",
                "--object-path",
                "/org/freedesktop/portal/desktop",
                "--method",
                "org.freedesktop.portal.Settings.Read",
                "org.freedesktop.appearance",
                "color-scheme",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        super::parse_color_scheme(&String::from_utf8_lossy(&output.stdout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_values_map_to_the_setting_and_back() {
        assert_eq!(Appearance::default(), Appearance::System);
        assert_eq!(Appearance::from_value(None), Appearance::System);
        assert_eq!(Appearance::from_value(Some("system")), Appearance::System);
        assert_eq!(Appearance::from_value(Some("light")), Appearance::Light);
        assert_eq!(Appearance::from_value(Some("dark")), Appearance::Dark);
        assert_eq!(Appearance::from_value(Some("neon")), Appearance::System);
        for setting in [Appearance::System, Appearance::Light, Appearance::Dark] {
            assert_eq!(Appearance::from_value(Some(setting.as_value())), setting);
        }
    }

    #[test]
    fn preference_follows_the_setting_and_the_portal() {
        assert_eq!(
            preference_for(Appearance::Light, Some(egui::Theme::Dark)),
            egui::ThemePreference::Light
        );
        assert_eq!(
            preference_for(Appearance::Dark, None),
            egui::ThemePreference::Dark
        );
        assert_eq!(
            preference_for(Appearance::System, None),
            egui::ThemePreference::System
        );
        assert_eq!(
            preference_for(Appearance::System, Some(egui::Theme::Light)),
            egui::ThemePreference::Light
        );
        assert_eq!(
            preference_for(Appearance::System, Some(egui::Theme::Dark)),
            egui::ThemePreference::Dark
        );
    }

    #[test]
    fn apply_sets_explicit_preferences() {
        let ctx = egui::Context::default();
        apply(&ctx, Appearance::Dark);
        assert_eq!(
            ctx.options(|options| options.theme_preference),
            egui::ThemePreference::Dark
        );
        apply(&ctx, Appearance::Light);
        assert_eq!(
            ctx.options(|options| options.theme_preference),
            egui::ThemePreference::Light
        );
    }

    /// The portal variant is a uint32: 0 = no preference, 1 = dark, 2 = light.
    #[test]
    fn portal_color_scheme_is_parsed() {
        assert_eq!(
            parse_color_scheme("(<<uint32 1>>,)"),
            Some(egui::Theme::Dark)
        );
        assert_eq!(
            parse_color_scheme("(<<uint32 2>>,)"),
            Some(egui::Theme::Light)
        );
        assert_eq!(parse_color_scheme("(<<uint32 0>>,)"), None);
        assert_eq!(parse_color_scheme("nonsense"), None);
        assert_eq!(parse_color_scheme(""), None);
    }

    #[test]
    fn appearance_labels_switch_with_the_language() {
        use crate::i18n::{Lang, lang_lock, set_lang};

        let _guard = lang_lock();
        set_lang(Lang::German);
        assert_eq!(Appearance::System.label(), "System");
        assert_eq!(Appearance::Light.label(), "Hell");
        assert_eq!(Appearance::Dark.label(), "Dunkel");
        set_lang(Lang::English);
        assert_eq!(Appearance::Light.label(), "Light");
        assert_eq!(Appearance::Dark.label(), "Dark");
    }
}
