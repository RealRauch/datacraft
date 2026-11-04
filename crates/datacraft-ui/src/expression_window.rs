//! Expression window (step 41.2, item M46).
//!
//! A floating window following the SQL-window pattern: presence comes
//! from the snapshot, the editor text is presentation state, and running
//! dispatches `RunExpression` without row context through the command
//! bus — the same command agents send. The window renders the result
//! value or the precise failure (row-context evaluation is a documented
//! follow-up, BACKLOG M46).

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;
use crate::i18n::t;
use crate::theme::tokens;

/// Renders the floating expression window (when open).
pub fn expression_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().expression_window_open {
        return;
    }
    let tokens = tokens(ui.visuals());
    let mut close = false;
    let mut run = false;
    let mut text = app.expression_text().to_owned();
    let output = app.expression_output().map(str::to_owned);
    let error = app.expression_error().map(str::to_owned);

    egui::Window::new(t("Expression"))
        .id(egui::Id::unique("datacraft.expression"))
        .default_width(420.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("Expression"));
                if ui.small_button(t("Close")).clicked() {
                    close = true;
                }
            });
            let editor = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .hint_text("[qty] * 1.19"),
            );
            if editor.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                run = true;
            }
            ui.horizontal(|ui| {
                if ui.button(t("Run")).clicked() {
                    run = true;
                }
                ui.weak(t("evaluates over an empty scope (no row context)"));
            });
            if let Some(output) = &output {
                ui.strong(format!("= {output}"));
            }
            if let Some(reason) = &error {
                ui.colored_label(tokens.danger, reason);
            }
        });

    app.set_expression_text(text);
    if run {
        app.run_expression();
    }
    if close {
        app.dispatch(Command::ToggleExpressionWindow);
    }
}
