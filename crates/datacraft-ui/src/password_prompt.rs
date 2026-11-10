//! Database-password prompt (step 16.2, item M5).
//!
//! Paint-only modal over the UI-owned prompt state; the retry dispatches the
//! same `OpenDatabase` command the ribbon, CLI and MCP use.

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use eframe::egui;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::theme::tokens;

/// Rejection-message key, kept as a key so a language switch re-renders
/// the open prompt live (the parity test consumes the constant).
pub(crate) const WRONG_PASSWORD: &str = "Wrong password — try again";

/// State of the database-password prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordPrompt {
    /// The file waiting for its password.
    pub path: std::path::PathBuf,
    /// Current password input.
    pub input: String,
    /// Translation key of the rejection message after a wrong attempt
    /// (kept as a key so a language switch re-renders it live).
    pub error: Option<&'static str>,
}

impl App {
    /// The open password prompt, if any.
    pub fn password_prompt(&self) -> Option<&PasswordPrompt> {
        self.password_prompt.as_ref()
    }

    /// Replaces the prompt's password input.
    pub fn set_password_input(&mut self, input: impl Into<String>) {
        if let Some(prompt) = &mut self.password_prompt {
            prompt.input = input.into();
        }
    }

    /// Retries the pending open with the typed password.
    pub fn submit_password(&mut self) {
        let Some(prompt) = self.password_prompt.as_ref() else {
            return;
        };
        let command = Command::OpenDatabase {
            path: prompt.path.clone(),
            password: Some(prompt.input.clone()),
            source: None,
        };
        self.dispatch(command);
    }

    /// Dismisses the password prompt without opening the file.
    pub fn cancel_password(&mut self) {
        self.password_prompt = None;
    }

    /// Applies the password events of one dispatch (step 16.2, M5): a
    /// protected file raises the prompt; a rejection keeps it open with the
    /// previous input; a successful open closes it.
    pub(crate) fn handle_password_events(&mut self, events: &[Event]) {
        if let Some(path) = events.iter().find_map(|event| match event {
            Event::PasswordRequired { path } => Some(path.clone()),
            _ => None,
        }) {
            self.password_prompt = Some(PasswordPrompt {
                path,
                input: String::new(),
                error: None,
            });
        }
        if let Some(path) = events.iter().find_map(|event| match event {
            Event::PasswordRejected { path } => Some(path.clone()),
            _ => None,
        }) {
            let input = self
                .password_prompt
                .as_ref()
                .map(|prompt| prompt.input.clone())
                .unwrap_or_default();
            self.password_prompt = Some(PasswordPrompt {
                path,
                input,
                error: Some(WRONG_PASSWORD),
            });
        }
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseOpened { .. }))
        {
            self.password_prompt = None;
        }
    }
}

/// Renders the modal when a protected database was opened without a password.
pub fn password_prompt(app: &mut App, ui: &mut egui::Ui) {
    let Some(prompt) = app.password_prompt().cloned() else {
        return;
    };
    let tokens = tokens(ui.visuals());
    let mut input = prompt.input.clone();
    let mut submit = false;
    let mut cancel = false;
    egui::Window::new(t("Database password"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label(tr(
                "{path} is password-protected.",
                &[("path", &prompt.path.display().to_string())],
            ));
            ui.add_space(4.0);
            let response = ui.add(
                egui::TextEdit::singleline(&mut input)
                    .password(true)
                    .desired_width(240.0)
                    .hint_text(t("password")),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            if let Some(error) = prompt.error {
                ui.colored_label(tokens.danger, t(error));
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(t("Open")).clicked() {
                    submit = true;
                }
                if ui.button(t("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    if input != prompt.input {
        app.set_password_input(input);
    }
    if submit {
        app.submit_password();
    }
    if cancel {
        app.cancel_password();
    }
}
