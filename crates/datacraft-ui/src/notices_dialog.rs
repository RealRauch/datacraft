//! Messages dialog (step 68.3, item M76).
//!
//! Lists the bounded notice/error history newest first; the status bar
//! shows only the latest entry. Pure UI state — clearing touches nothing
//! in core.

use eframe::egui;

use crate::app::App;
use crate::app::notices::Severity;
use crate::i18n::t;
use crate::theme::tokens;

/// Renders the dialog while it is open.
pub fn notices_dialog(app: &mut App, ui: &mut egui::Ui) {
    if !app.notices_open() {
        return;
    }
    let entries: Vec<(Severity, String, u64)> = app
        .notice_entries()
        .map(|entry| (entry.severity, entry.text.clone(), entry.seq))
        .collect();
    let tokens = tokens(ui.visuals());
    let mut open = true;
    let mut clear = false;
    egui::Window::new(t("Messages"))
        .open(&mut open)
        .resizable(true)
        .default_width(460.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(460.0, 320.0),
        ))
        .show(ui.ctx(), |ui| {
            if entries.is_empty() {
                ui.weak(t("No messages yet."));
            } else {
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (severity, text, seq) in &entries {
                            let color = if *severity == Severity::Error {
                                tokens.danger
                            } else {
                                tokens.accent
                            };
                            ui.push_id(seq, |ui| {
                                ui.colored_label(color, text);
                            });
                            ui.separator();
                        }
                    });
            }
            ui.separator();
            if ui.button(t("Clear")).clicked() {
                clear = true;
            }
        });
    if clear {
        app.clear_notices();
    }
    if !open {
        app.close_notices();
    }
}
