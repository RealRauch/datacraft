//! Native file and message dialogs (step 38.3, item M43).
//!
//! `rfd` drives the platform dialogs (Windows common dialogs, NSOpenPanel,
//! GTK portals) synchronously — a modal OS call, not an async runtime. The
//! [`DialogHost`] seam keeps the headless kittest in charge: tests install a
//! [`ScriptedDialogs`] host whose answers are queued, so every flow stays
//! label-driven and asserts on the dispatched commands.
//!
//! Two fences, decided in M43: native dialogs speak the OS language (outside
//! the i18n catalog by design — Excel behaves the same), while the prompt
//! texts we render ourselves go through `t()`/`tr()`; and the "No COM"
//! principle refers to the data path, not OS shell integration (rfd drives
//! the Windows common-dialog API the same way winit drives the windowing
//! API).

use std::path::PathBuf;

/// The dialog seam: file pickers and destructive-action confirmations.
pub trait DialogHost {
    /// A native open-file picker; `None` when the user cancels.
    fn pick_open_file(&mut self) -> Option<PathBuf>;
    /// A native open-folder picker (the CSV directory driver opens a
    /// directory as a database).
    fn pick_open_folder(&mut self) -> Option<PathBuf>;
    /// A native save-file picker seeded with `suggested`.
    fn pick_save_file(&mut self, suggested: &str) -> Option<PathBuf>;
    /// A native Ok/Cancel confirmation for a destructive action.
    fn confirm(&mut self, title: &str, message: &str) -> bool;
}

/// The real host: platform dialogs through `rfd`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeDialogs;

impl DialogHost for NativeDialogs {
    fn pick_open_file(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }

    fn pick_open_folder(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    fn pick_save_file(&mut self, suggested: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_file_name(suggested).save_file()
    }

    fn confirm(&mut self, title: &str, message: &str) -> bool {
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(message)
            .set_buttons(rfd::MessageButtons::OkCancel)
            .set_level(rfd::MessageLevel::Warning)
            .show()
            == rfd::MessageDialogResult::Ok
    }
}

/// Scripted host for the headless suite: answers are queued per dialog kind
/// and consumed in order; an empty queue answers "cancel"/"no".
#[derive(Debug, Default)]
pub struct ScriptedDialogs {
    files: Vec<Option<PathBuf>>,
    folders: Vec<Option<PathBuf>>,
    saves: Vec<Option<PathBuf>>,
    confirms: Vec<bool>,
}

impl ScriptedDialogs {
    /// Starts a script.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one open-file answer.
    pub fn file(mut self, answer: Option<&str>) -> Self {
        self.files.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one open-folder answer.
    pub fn folder(mut self, answer: Option<&str>) -> Self {
        self.folders.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one save-file answer.
    pub fn save(mut self, answer: Option<&str>) -> Self {
        self.saves.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one confirmation answer.
    pub fn confirm(mut self, answer: bool) -> Self {
        self.confirms.push(answer);
        self
    }
}

impl DialogHost for ScriptedDialogs {
    fn pick_open_file(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() {
            None
        } else {
            self.files.remove(0)
        }
    }

    fn pick_open_folder(&mut self) -> Option<PathBuf> {
        if self.folders.is_empty() {
            None
        } else {
            self.folders.remove(0)
        }
    }

    fn pick_save_file(&mut self, _suggested: &str) -> Option<PathBuf> {
        if self.saves.is_empty() {
            None
        } else {
            self.saves.remove(0)
        }
    }

    fn confirm(&mut self, _title: &str, _message: &str) -> bool {
        if self.confirms.is_empty() {
            false
        } else {
            self.confirms.remove(0)
        }
    }
}
