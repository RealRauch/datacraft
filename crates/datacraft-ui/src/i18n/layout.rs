//! Catalog layout checks (step 18.4, item I3; ADR-4, ARCHITECTURE §15).
//!
//! German strings are longer than their English sources. These tests
//! measure them with egui's real font metrics and enforce the fixed layout
//! slots: ribbon group captions sit in a narrow column and the status bar
//! must keep the record-navigation box reachable at the minimum window
//! width.

use eframe::egui;

use datacraft_core::ribbon::ribbon_spec;

use super::{Lang, translate};

/// Ribbon group column width (`ribbon.rs` sets the same minimum).
const GROUP_WIDTH: f32 = 120.0;
/// Breathing room inside the group column.
const GROUP_PADDING: f32 = 8.0;
/// Status-bar budget at the minimum window width (960 px) minus the
/// record-navigation box and chrome.
const STATUS_BUDGET: f32 = 640.0;

#[test]
fn german_ribbon_group_captions_fit_their_column() {
    let mut widths = Vec::new();
    run(|ctx| {
        let font = egui::FontId::new(
            crate::theme::fonts::CAPTION.size,
            egui::FontFamily::Proportional,
        );
        for tab in &ribbon_spec().tabs {
            for group in &tab.groups {
                let label = translate(Lang::German, group.label);
                widths.push((label, width_of(ctx, label, font.clone())));
            }
        }
    });
    for (label, width) in widths {
        assert!(
            width <= GROUP_WIDTH - GROUP_PADDING,
            "{label:?} needs {width:.1} px, budget {} px",
            GROUP_WIDTH - GROUP_PADDING
        );
    }
}

#[test]
fn german_status_lines_fit_the_minimum_window() {
    let mut widths = Vec::new();
    run(|ctx| {
        let font = egui::FontId::new(
            crate::theme::fonts::CHROME.size,
            egui::FontFamily::Proportional,
        );
        let plural = translate(Lang::German, crate::shell::STATUS_TABLES)
            .replace("{database}", "shop")
            .replace("{format}", "Jet4")
            .replace("{count}", "12");
        let single = translate(Lang::German, crate::shell::STATUS_TABLE)
            .replace("{database}", "shop")
            .replace("{format}", "Jet4")
            .replace("{count}", "1");
        let open = translate(Lang::German, "No database open").to_owned();
        for label in [plural, single, open] {
            let width = width_of(ctx, &label, font.clone());
            widths.push((label, width));
        }
    });
    for (label, width) in widths {
        assert!(
            width <= STATUS_BUDGET,
            "{label:?} needs {width:.1} px, budget {STATUS_BUDGET} px"
        );
    }
}

/// Runs one egui pass so the shipped fonts are laid out and measurable.
fn run(measure: impl FnMut(&egui::Context)) {
    let ctx = egui::Context::default();
    // Measure against the shipped typography (step 20.2): the budget must
    // reflect Inter and the rung scale, not egui's defaults.
    crate::theme::fonts::install(&ctx);
    crate::theme::apply(&ctx);
    let mut output = ctx.run_pass(egui::RawInput::default(), measure);
    // Headless: there is no painter that would apply the font-atlas delta.
    output.textures_delta.clear();
}

fn width_of(ctx: &egui::Context, text: &str, font: egui::FontId) -> f32 {
    ctx.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(text.to_owned(), font, egui::Color32::BLACK)
            .size()
            .x
    })
}
