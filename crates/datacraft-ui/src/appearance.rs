//! Appearance setting (step 20.4, item U4; ADR-6).
//!
//! `Appearance { Light, Dark, System }` (default `System`), persisted in
//! `ui.json` and applied as egui's theme preference. Both palettes are
//! installed by `theme::apply`; `theme::tokens` picks the set per frame from
//! `visuals.dark_mode`. On Linux, where winit reports no system theme, the
//! XDG desktop portal's `color-scheme` is polled (the model's
//! `desktop_theme.rs` pattern) — `parse_color_scheme` is the tested seam.

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
        assert_eq!(parse_color_scheme("(<<uint32 1>>,)"), Some(egui::Theme::Dark));
        assert_eq!(parse_color_scheme("(<<uint32 2>>,)"), Some(egui::Theme::Light));
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
