//! Glyph coverage contract (step 88.4, item T11).
//!
//! egui paints a character no installed face covers as tofu; the project
//! hit it twice (F12 combo caret, H53 ✕ U+2715) and fixed both by
//! painting instead of typing. This test walks every string the UI can
//! render from the catalogs — the English keys and the German values —
//! plus a sample of hostile object/file names, and asks the installed
//! font stack whether a glyph exists. A miss fails naming the language,
//! key and code point. Hostile code points the shipped faces deliberately
//! do not carry live on [`ACCEPTED_MISSING`]; every entry is pinned as
//! genuinely missing so the list stays honest and shrinks when coverage
//! grows. Headless: one `egui::Context`, no GPU.

use eframe::egui;

use super::de;
use crate::theme::fonts;

/// The only family that matters for coverage: every other family's face
/// chain is a superset of it (Inter Medium/SemiBold fall back to
/// Inter Regular, then egui's defaults).
fn probe_font_id() -> egui::FontId {
    egui::FontId::new(12.0, egui::FontFamily::Proportional)
}

/// Runs the reader against the headless installed font stack: one pass
/// so the definitions take effect.
fn installed_fonts<R>(reader: impl FnOnce(&mut egui::epaint::text::FontsView<'_>) -> R) -> R {
    let ctx = egui::Context::default();
    fonts::install(&ctx);
    ctx.begin_pass(egui::RawInput::default());
    ctx.fonts_mut(|fonts| reader(fonts))
}

/// Hostile-name sample: what users can legitimately put into object,
/// file and database names and into formatted values.
const HOSTILE_NAMES: &[&str] = &[
    "Kundendaten ÄÖÜ äöü ß", // German full alphabet
    "café naïve Zürich",     // accents the catalogs share
    "Tabelle±µ",             // technical symbols in values
    "表テーブル",            // CJK object names
    "🚀🔥",                  // emoji object names
    "Ωç√",                   // misc technical
];

/// Code points the shipped faces deliberately do not cover. Each entry
/// is pinned as genuinely missing; a font update that adds coverage
/// fails that pin until the entry is removed.
const ACCEPTED_MISSING: &[char] = &[
    '表', 'テ', 'ー', 'ブ', 'ル', // CJK object names
    '🚀', '🔥', // emoji object names
    '√',  // technical symbols
];

/// RED evidence only — an injected string the shipped stack cannot
/// cover; removed at GREEN (step 88.4, T11, H53's U+2715 evidence).
const INJECTED_MISSING: &[&str] = &["injected RED string ✕"];

/// Every catalog string renders without tofu in both languages.
#[test]
fn every_catalog_string_is_fully_covered_in_both_languages() {
    let missing = installed_fonts(|fonts| {
        let font_id = probe_font_id();
        let mut missing = Vec::new();
        let mut walk =
            |lang: &str, key: &str, text: &str, fonts: &mut egui::epaint::text::FontsView<'_>| {
                for c in text.chars().filter(|c| !c.is_control()) {
                    if !fonts.has_glyph(&font_id, c) {
                        missing.push(format!(
                            "{lang} key {key:?}: {:?} U+{:04X} has no glyph",
                            c, c as u32
                        ));
                    }
                }
            };
        for (english, german) in de::entries() {
            walk("en", english, english, fonts);
            walk("de", english, german, fonts);
        }
        for injected in INJECTED_MISSING {
            walk("injected", injected, injected, fonts);
        }
        missing
    });
    assert!(
        missing.is_empty(),
        "the shipped font stack misses glyphs the UI renders:\n{}",
        missing.join("\n")
    );
}

/// Hostile names carry only the accepted-missing glyphs, and every
/// accepted entry is genuinely missing today (the allowlist stays
/// honest).
#[test]
fn hostile_names_carry_only_the_accepted_missing_glyphs() {
    let missing = installed_fonts(|fonts| {
        let font_id = probe_font_id();
        let mut missing = Vec::new();
        for name in HOSTILE_NAMES {
            for c in name.chars().filter(|c| !c.is_control() && *c != ' ') {
                if !fonts.has_glyph(&font_id, c) && !ACCEPTED_MISSING.contains(&c) {
                    missing.push(format!(
                        "hostile name {name:?}: {:?} U+{:04X} has no glyph",
                        c, c as u32
                    ));
                }
            }
        }
        missing
    });
    assert!(
        missing.is_empty(),
        "the shipped font stack misses glyphs beyond the accepted list:\n{}",
        missing.join("\n")
    );

    let covered = installed_fonts(|fonts| {
        let font_id = probe_font_id();
        ACCEPTED_MISSING
            .iter()
            .copied()
            .filter(|c| fonts.has_glyph(&font_id, *c))
            .collect::<Vec<_>>()
    });
    assert!(
        covered.is_empty(),
        "remove these from ACCEPTED_MISSING — the shipped stack now covers them: {covered:?}"
    );
}
