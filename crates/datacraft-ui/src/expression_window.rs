//! Expression window (step 41.2, item M46).
//!
//! A tool window following the SQL-window pattern: presence comes from
//! the snapshot, the editor text is presentation state, and running
//! dispatches `RunExpression` without row context through the command
//! bus — the same command agents send. The window renders the result
//! value or the precise failure (row-context evaluation is a documented
//! follow-up, BACKLOG M46). Since the dock rollout (step 93.2, item
//! M125) the pane docks through the pane host right/bottom and keeps the
//! floating window while it floats.

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;
use crate::i18n::t;
use crate::panes::{DockMode, dock_menu};
use crate::theme::tokens;

/// Renders the floating expression window (when open and not docked).
pub fn expression_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().expression_window_open {
        return;
    }
    if app.pane_geometry("expression").mode != DockMode::Float {
        return; // the dock host's panel renders the content (M125)
    }
    let mut close = false;
    let response = egui::Window::new(t("Expression"))
        .id(egui::Id::unique("datacraft.expression".to_owned()))
        .default_width(420.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(420.0, 320.0),
        ))
        .show(ui, |ui| {
            let editor_label = expression_header(&mut *app, ui, &mut close);
            expression_body(app, ui, &editor_label);
        });
    // The floating window's outer rect rides the layout probe (the seam
    // the dock/float pins assert on).
    if let Some(response) = &response {
        crate::layout_probe::record("expression.window", response.response.rect);
    }
    if close {
        app.dispatch(Command::ToggleExpressionWindow);
    }
}

/// The docked pane content — the host header carries title, dock menu
/// and close (step 93.2, item M125); the editor keeps its label.
pub fn expression_pane_content(app: &mut App, ui: &mut egui::Ui) {
    let label = ui.label(t("Expression"));
    expression_body(app, ui, &label);
}

/// The floating window's header row: the editor's screen-reader label
/// with the dock menu on the left (the pilot's convention), the close
/// right-aligned and still the rightmost control (step 91.9, item L60).
/// Returns the label response the body hangs the editor on.
fn expression_header(app: &mut App, ui: &mut egui::Ui, close: &mut bool) -> egui::Response {
    ui.horizontal(|ui| {
        let editor_label = ui.label(t("Expression"));
        dock_menu(&mut *app, ui, "expression");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_response = ui.small_button(t("Close"));
            crate::layout_probe::record("expression.close", close_response.rect);
            if close_response.clicked() {
                *close = true;
            }
        });
        editor_label
    })
    .inner
}

/// The shared body: editor, the optional row context (step 60.1, item
/// L27), the run row and the result or the precise failure. `editor_label`
/// names the editor for screen readers.
fn expression_body(app: &mut App, ui: &mut egui::Ui, editor_label: &egui::Response) {
    let tokens = tokens(ui.visuals());
    let mut run = false;
    let mut context_changed = false;
    let mut text = app.expression_text().to_owned();
    let output = app.expression_output().map(str::to_owned);
    let error = app.expression_error().map(str::to_owned);
    let mut context = app.expression_context().cloned();
    let mut picked: Option<Option<(String, String)>> = None;
    let tables: Vec<String> = app
        .snapshot()
        .database
        .as_ref()
        .map(|db| db.tables.iter().map(|table| table.name.clone()).collect())
        .unwrap_or_default();

    let editor = ui.add(
        egui::TextEdit::singleline(&mut text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .hint_text("[qty] * 1.19"),
    );
    editor.clone().labelled_by(editor_label.id);
    if editor.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        run = true;
    }
    // The optional row context (step 60.1, item L27): a table plus the
    // zero-based record index — with one set, field names resolve against
    // that record.
    ui.horizontal(|ui| {
        egui::ComboBox::from_label(t("Row context"))
            .selected_text(match &context {
                None => t("no row context").to_owned(),
                Some((table, row)) => format!("{table} · {row}"),
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(context.is_none(), t("no row context"))
                    .clicked()
                {
                    picked = Some(None);
                }
                for table in &tables {
                    let is_current = context
                        .as_ref()
                        .is_some_and(|(current, _)| current.eq_ignore_ascii_case(table));
                    if ui.selectable_label(is_current, table).clicked() {
                        let row = context
                            .as_ref()
                            .map_or_else(|| "0".to_owned(), |(_, row)| row.clone());
                        picked = Some(Some((table.clone(), row)));
                    }
                }
            });
    });
    if let Some((_, row_text)) = &mut context {
        ui.horizontal(|ui| {
            let row_label = ui.label(t("Row"));
            let field = ui.add(egui::TextEdit::singleline(row_text).desired_width(60.0));
            field.labelled_by(row_label.id);
            ui.weak(t("0 = first record"));
        });
        context_changed = true;
    }
    ui.horizontal(|ui| {
        if ui.button(t("Run")).clicked() {
            run = true;
        }
        if context.is_none() {
            ui.weak(t("evaluates over an empty scope (no row context)"));
        }
    });
    if let Some(output) = &output {
        ui.strong(format!("= {output}"));
    }
    if let Some(reason) = &error {
        ui.colored_label(tokens.danger, reason);
    }

    if let Some(new_context) = picked {
        app.set_expression_context(new_context);
    } else if context_changed {
        app.set_expression_context(context);
    }
    app.set_expression_text(text);
    if run {
        app.run_expression();
    }
}
