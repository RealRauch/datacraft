//! Shortcuts help page (step 68.4, item M77).
//!
//! One localized table rendered from the exhaustive `keys::SHORTCUT_ROWS`,
//! plus the key-tip note; F1 toggles it.

use eframe::egui;

use crate::app::App;
use crate::i18n::t;
use crate::keys::SHORTCUT_ROWS;

/// Renders the page while it is open.
pub fn shortcuts_help(app: &mut App, ui: &mut egui::Ui) {
    if !app.shortcuts_help_open() {
        return;
    }
    let mut open = true;
    let mut close = false;
    egui::Window::new(t("Keyboard Shortcuts"))
        .open(&mut open)
        .resizable(false)
        .default_width(460.0)
        .show(ui.ctx(), |ui| {
            egui::Grid::new("shortcuts.help.grid")
                .num_columns(2)
                .striped(true)
                .spacing([18.0, 4.0])
                .show(ui, |ui| {
                    for (_, keys, action) in SHORTCUT_ROWS {
                        ui.strong(*keys);
                        ui.label(t(action));
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.label(t(
                "Alt reveals the ribbon key tips; Esc cancels the current edit or closes the dialog.",
            ));
            ui.separator();
            if ui.button(t("Close")).clicked() {
                close = true;
            }
        });
    if close || !open {
        app.close_shortcuts_help();
    }
}
