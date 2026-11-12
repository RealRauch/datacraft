//! Object rename dialog (phase 72, step 72.4; item M81).
//!
//! The navigation pane's context menu and F2 open one small window seeded
//! with the current name; Rename dispatches exactly one `RenameObject`
//! command. A refusal keeps the dialog open with the precise reason — the
//! buffer survives, like the view editor (step 71.2).

use eframe::egui;

use datacraft_core::ObjectId;
use datacraft_core::command::Command;
use datacraft_core::event::Event;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::theme::tokens;

/// The open dialog's draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
    /// The object being renamed (kind + canonical current name).
    pub object: ObjectId,
    /// The edited new name.
    pub name: String,
    /// The last refusal, rendered verbatim while the dialog stays open.
    pub error: Option<String>,
    /// Whether the text field still needs the initial focus.
    pub focus_pending: bool,
}

impl App {
    /// Opens the dialog on one object, seeded with its current name.
    pub fn open_rename_dialog(&mut self, object: ObjectId) {
        self.rename_draft = Some(RenameDraft {
            name: object.name.clone(),
            object,
            error: None,
            focus_pending: true,
        });
    }

    /// The open dialog draft.
    pub fn rename_draft(&self) -> Option<&RenameDraft> {
        self.rename_draft.as_ref()
    }

    /// Replaces the draft name.
    pub fn set_rename_draft_name(&mut self, name: &str) {
        if let Some(draft) = &mut self.rename_draft {
            draft.name = name.to_owned();
        }
    }

    /// Closes the dialog without renaming.
    pub fn close_rename_dialog(&mut self) {
        self.rename_draft = None;
    }

    /// Consumes the initial-focus request (one frame only).
    pub fn take_rename_focus(&mut self) -> bool {
        match &mut self.rename_draft {
            Some(draft) => std::mem::take(&mut draft.focus_pending),
            None => false,
        }
    }

    /// Dispatches the draft; a refusal keeps the dialog and buffer alive.
    pub fn submit_rename_draft(&mut self) {
        let Some(draft) = self.rename_draft.clone() else {
            return;
        };
        let events = self.dispatch(Command::RenameObject {
            object: draft.object.clone(),
            new_name: draft.name.clone(),
        });
        let failure = events.iter().find_map(|event| match event {
            Event::Failed { code, reason } => Some((*code, reason.clone())),
            _ => None,
        });
        match failure {
            Some((code, reason)) => {
                if let Some(current) = &mut self.rename_draft {
                    current.error = Some(crate::failure_text::failure_text(code, &reason));
                }
            }
            None => self.rename_draft = None,
        }
    }
}

/// The rename window (rendered by the shell next to the other dialogs).
pub fn rename_dialog(app: &mut App, ui: &mut egui::Ui) {
    let Some(draft) = app.rename_draft().cloned() else {
        return;
    };
    let mut name = draft.name.clone();
    let mut open = true;
    let mut submit = false;
    let mut cancel = false;
    egui::Window::new(tr("Rename {name}", &[("name", &draft.object.name)]))
        .id(egui::Id::unique("datacraft.rename".to_owned()))
        .open(&mut open)
        .resizable(false)
        .default_width(360.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(360.0, 170.0),
        ))
        .show(ui, |ui| {
            ui.label(tr("Current name: {name}", &[("name", &draft.object.name)]));
            let label = ui.label(t("New name"));
            let field = ui.text_edit_singleline(&mut name).labelled_by(label.id);
            if app.take_rename_focus() {
                field.request_focus();
            }
            // Keep the buffer in the draft every frame: the widget edits the
            // local copy, and the draft is the source of truth for submit.
            if field.changed() {
                app.set_rename_draft_name(&name);
            }
            if let Some(error) = &draft.error {
                ui.colored_label(tokens(ui.visuals()).danger, error);
            }
            if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            ui.horizontal(|ui| {
                if ui.button(t("Rename")).clicked() {
                    submit = true;
                }
                if ui.button(t("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
    if submit {
        app.set_rename_draft_name(&name);
        app.submit_rename_draft();
    } else if cancel || !open {
        app.close_rename_dialog();
    }
}
