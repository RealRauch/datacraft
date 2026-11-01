//! ArtCraft theme (step 12.1, item H31-followup "Access fidelity").
//!
//! The palette is the app icon's, exactly three colours (see
//! `docs/brand/DATACRAFT-ICON.md`): ink `#0B0B0C`, paper `#EFE9DC` and
//! DataCraft teal `#0F8B7E` — the icon's full-bleed tile field. Derived
//! tones stay inside the grammar: paper tints for chrome surfaces, teal
//! tints for interaction states, ink at reduced alpha for secondary text
//! and hairlines. `apply` is idempotent and cheap to call every frame.

use eframe::egui;
use eframe::egui::Color32;

/// Contour, text, grid — the icon's outline colour.
pub const INK: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0C);
/// Content surfaces — the icon's paper figure.
pub const PAPER: Color32 = Color32::from_rgb(0xEF, 0xE9, 0xDC);
/// The theme colour — the icon's full-bleed tile background.
pub const TEAL: Color32 = Color32::from_rgb(0x0F, 0x8B, 0x7E);

/// Chrome strips (ribbon, status bar): a paper tint a step darker so the
/// document area reads as the bright surface.
pub const CHROME: Color32 = Color32::from_rgb(0xE3, 0xDA, 0xC6);
/// Elevated paper (document windows, hovered chrome rows).
pub const PAPER_LIGHT: Color32 = Color32::from_rgb(0xF8, 0xF4, 0xEA);
/// Ink at reading strength for secondary text and hairlines.
pub const INK_SOFT: Color32 = Color32::from_rgb(0x5A, 0x56, 0x4C);
/// Hairline (grid lines, separators) — ink at low alpha.
pub const HAIRLINE: Color32 = Color32::from_rgba_premultiplied(0x0B, 0x0B, 0x0C, 0x28);
/// Hover tint over chrome: teal blended into paper.
pub const TEAL_WASH: Color32 = Color32::from_rgb(0xDC, 0xE9, 0xE4);
/// Text on teal fills (headers, active tabs, primary buttons).
pub const ON_TEAL: Color32 = Color32::from_rgb(0xF8, 0xF4, 0xEA);

/// Applies the ArtCraft theme to the context (idempotent; mutates both
/// egui themes so the look is theme-independent).
pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;

        visuals.window_fill = PAPER;
        visuals.panel_fill = CHROME;
        visuals.extreme_bg_color = PAPER_LIGHT;
        visuals.faint_bg_color = PAPER;
        visuals.selection.bg_fill = TEAL;
        visuals.selection.stroke = egui::Stroke::new(1.0, TEAL);
        visuals.hyperlink_color = TEAL;
        visuals.override_text_color = Some(INK);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, HAIRLINE);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, INK);
        visuals.widgets.noninteractive.bg_fill = PAPER;

        let tint = |w: &mut egui::style::WidgetVisuals, bg: Color32, stroke: Color32| {
            w.bg_fill = bg;
            w.fg_stroke = egui::Stroke::new(1.0, stroke);
            w.bg_stroke = egui::Stroke::new(1.0, stroke);
        };
        tint(
            &mut visuals.widgets.inactive,
            PAPER_LIGHT,
            Color32::from_rgba_premultiplied(0x0B, 0x0B, 0x0C, 0x40),
        );
        tint(&mut visuals.widgets.hovered, TEAL_WASH, TEAL);
        tint(&mut visuals.widgets.active, TEAL, ON_TEAL);
        tint(&mut visuals.widgets.open, PAPER_LIGHT, INK_SOFT);

        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The model's neutral system, pinned value by value (step 20.1, U1).
    #[test]
    fn neutral_tokens_match_the_model() {
        let t = tokens(&egui::Visuals::light());
        assert_eq!(t.chrome, Color32::from_rgb(0xF0, 0xF0, 0xF0));
        assert_eq!(t.ribbon, Color32::from_rgb(0xF9, 0xF9, 0xF9));
        assert_eq!(t.canvas, Color32::from_rgb(0xE6, 0xE6, 0xE6));
        assert_eq!(t.panel, Color32::from_rgb(0xFA, 0xFA, 0xFA));
        assert_eq!(t.text, Color32::from_rgb(0x24, 0x24, 0x24));
        assert_eq!(t.text_muted, Color32::from_rgb(0x61, 0x61, 0x61));
        assert_eq!(t.text_disabled, Color32::from_rgb(0xA8, 0xA8, 0xA8));
        assert_eq!(t.border, Color32::from_rgb(0xE1, 0xE1, 0xE1));
        assert_eq!(t.border_strong, Color32::from_rgb(0xC8, 0xC8, 0xC8));
        assert_eq!(t.hover, Color32::from_rgb(0xE6, 0xE6, 0xE6));
        assert_eq!(t.pressed, Color32::from_rgb(0xD6, 0xD6, 0xD6));
    }

    /// ADR-6: every accent token is a teal derivative. The model's blue
    /// family is checked in explicitly so a future palette import fails.
    #[test]
    fn accent_tokens_are_teal_derivatives() {
        let t = tokens(&egui::Visuals::light());
        assert_eq!(t.accent, BRAND_TEAL);
        assert_eq!(t.accent, Color32::from_rgb(0x0F, 0x8B, 0x7E));
        assert_eq!(t.accent_text, Color32::from_rgb(0x0B, 0x6B, 0x60));
        assert_eq!(t.checked, Color32::from_rgb(0xDC, 0xE9, 0xE4));
        assert_eq!(t.on_accent, Color32::from_rgb(0xF8, 0xF4, 0xEA));
        assert_eq!(
            t.accent_faint,
            Color32::from_rgba_unmultiplied_const(0x0F, 0x8B, 0x7E, 0x24),
            "the cursor-row wash is the brand teal as a faint veil"
        );

        let model_blue = [
            Color32::from_rgb(0x3B, 0x5B, 0xDB),
            Color32::from_rgb(0x2B, 0x47, 0xB5),
            Color32::from_rgb(0x7A, 0x93, 0xF0),
            Color32::from_rgb(0x9C, 0xB0, 0xF5),
        ];
        for token in [
            t.accent,
            t.accent_text,
            t.checked,
            t.on_accent,
            t.accent_faint,
        ] {
            assert!(
                !model_blue.contains(&token),
                "accent token {token:?} is the model's blue, which ADR-6 rejects"
            );
        }
    }

    /// The brand icon's tile field and the UI accent stay the same colour.
    #[test]
    fn brand_icon_teal_is_the_accent() {
        let t = tokens(&egui::Visuals::light());
        assert_eq!(t.accent, BRAND_TEAL);
        assert_eq!(t.accent.r(), 0x0F);
        assert_eq!(t.accent.g(), 0x8B);
        assert_eq!(t.accent.b(), 0x7E);
    }

    /// Geometry values from the model (step 20.1, U1).
    #[test]
    fn geometry_matches_the_model() {
        assert_eq!(GEOMETRY.item_spacing, egui::vec2(6.0, 4.0));
        assert_eq!(GEOMETRY.button_padding, egui::vec2(8.0, 3.0));
        assert_eq!(GEOMETRY.window_corner_radius, egui::CornerRadius::same(8));
        assert_eq!(GEOMETRY.menu_corner_radius, egui::CornerRadius::same(6));
        assert_eq!(GEOMETRY.widget_corner_radius, egui::CornerRadius::same(4));
        assert_eq!(GEOMETRY.stroke_width, 1.0);
    }

    /// `apply` installs the token set into both egui themes — the UI paints
    /// from tokens, not from literals (acceptance U1).
    #[test]
    fn apply_installs_tokens_into_both_styles() {
        let ctx = egui::Context::default();
        apply(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let style = ctx.style_of(theme);
            let v = &style.visuals;
            let t = tokens(v);

            assert_eq!(v.panel_fill, t.panel);
            assert_eq!(v.window_fill, t.panel);
            assert_eq!(v.extreme_bg_color, t.panel);
            assert_eq!(v.faint_bg_color, t.canvas);
            assert_eq!(v.override_text_color, Some(t.text));
            assert_eq!(v.hyperlink_color, t.accent);
            assert_eq!(v.selection.bg_fill, t.checked);
            assert_eq!(v.selection.stroke.color, t.accent_text);
            assert_eq!(v.selection.stroke.width, GEOMETRY.stroke_width);

            assert_eq!(v.widgets.noninteractive.bg_fill, t.panel);
            assert_eq!(v.widgets.noninteractive.fg_stroke.color, t.text);
            assert_eq!(v.widgets.noninteractive.bg_stroke.color, t.border);
            assert_eq!(v.widgets.inactive.bg_fill, t.panel);
            assert_eq!(v.widgets.inactive.weak_bg_fill, t.panel);
            assert_eq!(v.widgets.inactive.bg_stroke.color, t.border);
            assert_eq!(v.widgets.hovered.bg_fill, t.hover);
            assert_eq!(v.widgets.hovered.bg_stroke.color, t.border_strong);
            assert_eq!(v.widgets.active.bg_fill, t.pressed);
            assert_eq!(v.widgets.open.bg_fill, t.panel);
            assert_eq!(v.widgets.open.bg_stroke.color, t.border_strong);

            for widget in [
                &v.widgets.noninteractive,
                &v.widgets.inactive,
                &v.widgets.hovered,
                &v.widgets.active,
                &v.widgets.open,
            ] {
                assert_eq!(widget.corner_radius, GEOMETRY.widget_corner_radius);
                assert_eq!(widget.fg_stroke.width, GEOMETRY.stroke_width);
                assert_eq!(widget.bg_stroke.width, GEOMETRY.stroke_width);
            }
            assert_eq!(v.window_corner_radius, GEOMETRY.window_corner_radius);
            assert_eq!(v.menu_corner_radius, GEOMETRY.menu_corner_radius);
            assert_eq!(style.spacing.item_spacing, GEOMETRY.item_spacing);
            assert_eq!(style.spacing.button_padding, GEOMETRY.button_padding);
        }
    }

    /// `apply` runs every frame; a second pass must not drift the style.
    #[test]
    fn apply_is_idempotent() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let once = ctx.style_of(egui::Theme::Light);
        apply(&ctx);
        let twice = ctx.style_of(egui::Theme::Light);
        assert_eq!(once.visuals.panel_fill, twice.visuals.panel_fill);
        assert_eq!(once.visuals.selection.bg_fill, twice.visuals.selection.bg_fill);
        assert_eq!(
            once.visuals.widgets.active.bg_fill,
            twice.visuals.widgets.active.bg_fill
        );
        assert_eq!(once.spacing.item_spacing, twice.spacing.item_spacing);
        assert_eq!(once.spacing.button_padding, twice.spacing.button_padding);
    }
}
