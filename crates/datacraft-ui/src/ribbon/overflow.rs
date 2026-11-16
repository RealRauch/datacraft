//! Ribbon group overflow (step 88.1, item M98; split from `ribbon.rs` to
//! keep the file below the size gate — L30).
//!
//! The fit plan comes from `datacraft_core::ribbon::fit_groups` (the H52
//! precedent: layout math lives in core). This module only measures the
//! natural group widths, reserves the control's chrome and paints the
//! overflow menu: the hidden groups' own action buttons stacked under
//! their captions — the same `action_button` the row paints, so disabled
//! states, tooltips and command dispatch stay identical by construction.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{GroupFit, RibbonGroup, fit_groups};

use crate::app::App;
use crate::i18n::t;
use crate::layout_probe;
use crate::theme::fonts;
use crate::theme::tokens;

use super::actions::action_button;

/// egui's default separator allocation in a horizontal row
/// (`SeparatorStyle::spacing` in egui's `default_style.rs`).
const SEPARATOR_WIDTH: f32 = 6.0;

/// Painted width of the overflow control (the localized "More" button).
const CONTROL_WIDTH: f32 = 64.0;

/// The overflow plan for one group row. `available` is the width the row
/// still offers after its leading inset; the separator chrome of every
/// group (item spacing before each element plus the separator's own
/// width) is subtracted here so the core fit math sees the real group
/// budget. The control's own spacing and width stay reserved by
/// `fit_groups`.
pub(super) fn plan(ui: &egui::Ui, widths: &[f32], spacing: f32) -> GroupFit {
    let groups = widths.len();
    let separators = SEPARATOR_WIDTH * groups as f32;
    let element_spacing = spacing * (2 * groups) as f32; // group + separator each
    let available = ui.available_width() - separators - element_spacing;
    fit_groups(widths, available, spacing, CONTROL_WIDTH)
}

/// Paints the overflow control and its menu: one captioned stack of the
/// hidden groups' action buttons. Returns the command a click collected.
pub(super) fn overflow_control(
    app: &mut App,
    ui: &mut egui::Ui,
    tab_id: &str,
    hidden: &[RibbonGroup],
) -> Option<Command> {
    let mut intent = None;
    let response = ui.menu_button(t("More"), |ui| {
        for group in hidden {
            ui.separator();
            let tokens = tokens(ui.visuals());
            ui.label(
                egui::RichText::new(t(group.label))
                    .size(fonts::CAPTION.size)
                    .extra_letter_spacing(fonts::CAPTION.tracking_points())
                    .color(tokens.text_muted),
            );
            for action in &group.actions {
                let id = format!("ribbon.{}.{}.{}", tab_id, group.id, action.id);
                if let Some(command) = ui
                    .push_id(id, |ui| {
                        action_button(app, ui, tab_id, group.id, action, None)
                    })
                    .inner
                {
                    intent = Some(command);
                    ui.close();
                }
            }
        }
    });
    layout_probe::record(&format!("ribbon.{tab_id}.overflow"), response.response.rect);
    response.response.on_hover_text(t("More ribbon actions"));
    intent
}
