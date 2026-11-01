//! Theme tokens (step 20.1, item U1; ADR-6 in `docs/ARCHITECTURE.md` §15).
//!
//! The look rebases on the WordCraft neutral system: chrome, ribbon, canvas
//! and panel surfaces are neutral greys, text and borders are neutral ramps,
//! and hover/pressed states stay neutral. The accent family is DataCraft's
//! brand teal — the app icon's full-bleed tile field — and every
//! accent-adjacent token derives from it; the model's blue family is
//! explicitly rejected (pinned by a token test). Constants name roles, never
//! literals. `apply` is idempotent and cheap to call every frame.

use eframe::egui;
use eframe::egui::Color32;

pub mod fonts;

/// The brand teal — the app icon's full-bleed tile field. Every accent
/// token derives from this colour (ADR-6).
pub const BRAND_TEAL: Color32 = Color32::from_rgb(0x0F, 0x8B, 0x7E);

/// Colour tokens for one appearance. Roles are regions and interaction
/// states, so paint sites never carry literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    /// Chrome strips (title/tab strip, status bar).
    pub chrome: Color32,
    /// Ribbon surfaces.
    pub ribbon: Color32,
    /// Document canvas (workspace background).
    pub canvas: Color32,
    /// Panels beside the canvas (navigation pane).
    pub panel: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text (ribbon group captions, hints).
    pub text_muted: Color32,
    /// Disabled text.
    pub text_disabled: Color32,
    /// Hairlines and widget outlines.
    pub border: Color32,
    /// Stronger outlines (hover, open menus).
    pub border_strong: Color32,
    /// Hover surface.
    pub hover: Color32,
    /// Pressed surface.
    pub pressed: Color32,
    /// Accent fill (datasheet header band, links).
    pub accent: Color32,
    /// Accent ink on light accent washes.
    pub accent_text: Color32,
    /// Checked/selection wash.
    pub checked: Color32,
    /// Text on accent fills.
    pub on_accent: Color32,
    /// Faint accent veil (cursor row).
    pub accent_faint: Color32,
    /// Error text.
    pub danger: Color32,
}

impl Tokens {
    /// The light appearance.
    pub const fn light() -> Self {
        Self {
            chrome: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            ribbon: Color32::from_rgb(0xF9, 0xF9, 0xF9),
            canvas: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            panel: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            text: Color32::from_rgb(0x24, 0x24, 0x24),
            text_muted: Color32::from_rgb(0x61, 0x61, 0x61),
            text_disabled: Color32::from_rgb(0xA8, 0xA8, 0xA8),
            border: Color32::from_rgb(0xE1, 0xE1, 0xE1),
            border_strong: Color32::from_rgb(0xC8, 0xC8, 0xC8),
            hover: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            pressed: Color32::from_rgb(0xD6, 0xD6, 0xD6),
            accent: BRAND_TEAL,
            accent_text: Color32::from_rgb(0x0B, 0x6B, 0x60),
            checked: Color32::from_rgb(0xDC, 0xE9, 0xE4),
            on_accent: Color32::from_rgb(0xF8, 0xF4, 0xEA),
            accent_faint: Color32::from_rgba_unmultiplied_const(0x0F, 0x8B, 0x7E, 0x24),
            danger: Color32::from_rgb(0xB0, 0x2A, 0x2A),
        }
    }
}

/// The active token set. Dark mode (step 20.4, U4) selects its pendant here,
/// from `visuals.dark_mode`.
pub fn tokens(_visuals: &egui::Visuals) -> &'static Tokens {
    &LIGHT
}

/// The light token set.
pub const LIGHT: Tokens = Tokens::light();

/// Geometry rungs from the model (step 20.1, U1): spacing, paddings, corner
/// radii and the 1 px stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// Gap between widgets.
    pub item_spacing: egui::Vec2,
    /// Padding inside buttons.
    pub button_padding: egui::Vec2,
    /// Window corner radius.
    pub window_corner_radius: egui::CornerRadius,
    /// Menu/popup corner radius.
    pub menu_corner_radius: egui::CornerRadius,
    /// Widget corner radius.
    pub widget_corner_radius: egui::CornerRadius,
    /// The single stroke rung.
    pub stroke_width: f32,
}

/// The model's geometry rungs.
pub const GEOMETRY: Geometry = Geometry {
    item_spacing: egui::Vec2::new(6.0, 4.0),
    button_padding: egui::Vec2::new(8.0, 3.0),
    window_corner_radius: egui::CornerRadius::same(8),
    menu_corner_radius: egui::CornerRadius::same(6),
    widget_corner_radius: egui::CornerRadius::same(4),
    stroke_width: 1.0,
};

/// Applies the token set to the context (idempotent; mutates both egui
/// themes so the look is theme-independent until dark mode lands in U4).
pub fn apply(ctx: &egui::Context) {
    let t = tokens(&ctx.global_style().visuals);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;

        visuals.window_fill = t.panel;
        visuals.panel_fill = t.panel;
        visuals.extreme_bg_color = t.panel;
        visuals.faint_bg_color = t.canvas;
        visuals.selection.bg_fill = t.checked;
        visuals.selection.stroke = egui::Stroke::new(GEOMETRY.stroke_width, t.accent_text);
        visuals.hyperlink_color = t.accent;
        visuals.override_text_color = Some(t.text);
        visuals.window_corner_radius = GEOMETRY.window_corner_radius;
        visuals.menu_corner_radius = GEOMETRY.menu_corner_radius;

        let rung =
            |widget: &mut egui::style::WidgetVisuals, bg: Color32, fg: Color32, stroke: Color32| {
                widget.bg_fill = bg;
                widget.weak_bg_fill = bg;
                widget.fg_stroke = egui::Stroke::new(GEOMETRY.stroke_width, fg);
                widget.bg_stroke = egui::Stroke::new(GEOMETRY.stroke_width, stroke);
                widget.corner_radius = GEOMETRY.widget_corner_radius;
            };
        rung(
            &mut visuals.widgets.noninteractive,
            t.panel,
            t.text,
            t.border,
        );
        rung(&mut visuals.widgets.inactive, t.panel, t.text, t.border);
        rung(
            &mut visuals.widgets.hovered,
            t.hover,
            t.text,
            t.border_strong,
        );
        rung(
            &mut visuals.widgets.active,
            t.pressed,
            t.text,
            t.border_strong,
        );
        rung(&mut visuals.widgets.open, t.panel, t.text, t.border_strong);

        style.spacing.item_spacing = GEOMETRY.item_spacing;
        style.spacing.button_padding = GEOMETRY.button_padding;
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
        assert_eq!(
            once.visuals.selection.bg_fill,
            twice.visuals.selection.bg_fill
        );
        assert_eq!(
            once.visuals.widgets.active.bg_fill,
            twice.visuals.widgets.active.bg_fill
        );
        assert_eq!(once.spacing.item_spacing, twice.spacing.item_spacing);
        assert_eq!(once.spacing.button_padding, twice.spacing.button_padding);
    }
}
