//! Embedded fonts and type rungs (step 20.2, item U2).
//!
//! Inter is the proportional UI face; its Medium and SemiBold cuts back the
//! named families `medium`/`semibold`, and JetBrains Mono renders the SQL
//! editor. egui's default faces stay behind them as fallback, so a missing
//! glyph still resolves. The type rungs pin the model's scale: Chrome 12.5
//! (−0.012 em), Control 11.5, Caption 11.0 (+0.018 em), plus egui's
//! Body/Button 12.5, Small 10.5 and Heading 17 semibold.

#[cfg(test)]
mod tests {
    use super::*;

    /// The rung scale from the model (step 20.2, U2).
    #[test]
    fn rungs_match_the_model() {
        assert_eq!(CHROME.size, 12.5);
        assert_eq!(CHROME.tracking_em, -0.012);
        assert_eq!(CONTROL.size, 11.5);
        assert_eq!(CONTROL.tracking_em, 0.0);
        assert_eq!(CAPTION.size, 11.0);
        assert_eq!(CAPTION.tracking_em, 0.018);
        assert!((CAPTION.tracking_points() - 0.198).abs() < 1e-6);
        assert!((CHROME.tracking_points() + 0.15).abs() < 1e-6);
        assert_eq!(BODY_SIZE, 12.5);
        assert_eq!(SMALL_SIZE, 10.5);
        assert_eq!(HEADING_SIZE, 17.0);
    }

    /// The embedded font files are the pinned OFL assets, not placeholders.
    #[test]
    fn embedded_font_assets_are_valid_ttf() {
        for (name, bytes) in [
            ("Inter-Regular.ttf", INTER_REGULAR),
            ("Inter-Medium.ttf", INTER_MEDIUM),
            ("Inter-SemiBold.ttf", INTER_SEMIBOLD),
            ("JetBrainsMono-Regular.ttf", JETBRAINS_MONO),
        ] {
            assert!(
                bytes.len() > 100_000,
                "{name} is implausibly small ({} bytes)",
                bytes.len()
            );
            assert_eq!(
                &bytes[..4],
                &[0x00, 0x01, 0x00, 0x00],
                "{name} is not a TrueType font"
            );
        }
    }

    /// Inter leads the proportional families; egui's faces stay as fallback.
    #[test]
    fn inter_leads_every_proportional_family() {
        let baseline = egui::FontDefinitions::default();
        let default_faces = baseline.families[&egui::FontFamily::Proportional].clone();
        let default_mono = baseline.families[&egui::FontFamily::Monospace].clone();

        let definitions = font_definitions();
        let proportional = &definitions.families[&egui::FontFamily::Proportional];
        assert_eq!(proportional[0], "inter_regular");
        assert_eq!(&proportional[1..], default_faces.as_slice());

        for (family, face) in [
            (FAMILY_MEDIUM, "inter_medium"),
            (FAMILY_SEMIBOLD, "inter_semibold"),
        ] {
            let faces = &definitions.families[&egui::FontFamily::Name(family.into())];
            assert_eq!(faces[0], face);
            assert_eq!(faces[1], "inter_regular");
            assert_eq!(&faces[2..], default_faces.as_slice());
        }

        let mono = &definitions.families[&egui::FontFamily::Monospace];
        assert_eq!(mono[0], "jetbrains_mono");
        assert_eq!(&mono[1..], default_mono.as_slice());

        for face in [
            "inter_regular",
            "inter_medium",
            "inter_semibold",
            "jetbrains_mono",
        ] {
            assert!(
                definitions.font_data.contains_key(face),
                "{face} is missing from the font data"
            );
        }
    }

    /// `install` registers the fonts and sets the rung scale in both themes.
    #[test]
    fn install_sets_the_type_scale_in_both_themes() {
        let ctx = egui::Context::default();
        install(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let style = ctx.style_of(theme);
            let body = style.text_styles[&egui::TextStyle::Body];
            assert_eq!(body.size, BODY_SIZE);
            assert_eq!(body.family, egui::FontFamily::Proportional);
            assert_eq!(style.text_styles[&egui::TextStyle::Button].size, BODY_SIZE);
            assert_eq!(style.text_styles[&egui::TextStyle::Small].size, SMALL_SIZE);

            let heading = style.text_styles[&egui::TextStyle::Heading];
            assert_eq!(heading.size, HEADING_SIZE);
            assert_eq!(
                heading.family,
                egui::FontFamily::Name(FAMILY_SEMIBOLD.into())
            );
            let mono = style.text_styles[&egui::TextStyle::Monospace];
            assert_eq!(mono.family, egui::FontFamily::Monospace);
            assert_eq!(mono.size, 12.0);
        }
    }

    /// A second install call (startup plus a language switch, say) is stable.
    #[test]
    fn install_is_idempotent() {
        let ctx = egui::Context::default();
        install(&ctx);
        install(&ctx);
        let style = ctx.style_of(egui::Theme::Light);
        assert_eq!(style.text_styles[&egui::TextStyle::Body].size, BODY_SIZE);
        assert_eq!(style.text_styles[&egui::TextStyle::Heading].size, HEADING_SIZE);
    }
}
