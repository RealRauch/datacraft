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

/// Paint helper: a teal-filled caption bar (datasheet headers, active tab
/// underline) — the icon's header-row grammar.
pub fn teal_fill(rect: egui::Rect, painter: &egui::Painter) {
    painter.rect_filled(rect, 0.0, TEAL);
}
