//! The parked dialog flows (step 81.2, item M97): each dialog request
//! stores its follow-up intent here, the per-frame poll runs it on the
//! answer — no flow blocks inside the frame that asked for the dialog.

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::dialogs::{DialogAnswer, DialogRequest};
use crate::i18n::t;

/// The parked follow-up of one dialog request (step 81.2, item M97):
/// what happens when the answer arrives on a later frame. Everything a
/// confirmation needs is captured when the request parks, so a late
/// answer runs exactly the flow the user clicked.
#[derive(Debug, Clone)]
pub(crate) enum DialogIntent {
    /// The picked path lands in the open-database buffer.
    FillOpenPath,
    /// The picked path lands in the save-copy buffer.
    FillSaveCopyPath,
    /// The picked path lands in the new-database buffer.
    FillNewDatabasePath,
    /// The picked path becomes the import source; the preview dispatches.
    ImportSource,
    /// A confirmed overwrite dispatches the save-copy.
    SaveCopy {
        path: String,
        password: Option<String>,
    },
    /// A confirmed deletion dispatches the row delete.
    DeleteRow { table: String, row: usize },
    /// A confirmed deletion stages into the tab's edit buffer (step
    /// 85.2, item H61) — the batch applies on Ctrl+S.
    StageRowDelete { tab_id: TabId, slot: usize },
    /// A confirmed deletion dispatches a prepared object command.
    DeleteObject { command: Command },
    /// The picked path receives the report's PDF export (phase 97, M128).
    ExportReportPdf { report: String },
}

impl App {
    /// Parks a dialog request with its follow-up (step 81.2, item M97).
    /// One dialog at a time: while a request is unanswered, further
    /// requests are ignored — the open dialog owns the screen and is the
    /// visible feedback; a queue of dialogs behind it would surprise.
    pub(crate) fn request_dialog(&mut self, request: DialogRequest, intent: DialogIntent) {
        if self.pending_dialog.is_some() {
            return;
        }
        self.pending_dialog = Some(intent);
        self.dialogs.request(request);
    }

    /// Whether a dialog request is parked (test pins).
    #[cfg(test)]
    pub(crate) fn has_pending_dialog(&self) -> bool {
        self.pending_dialog.is_some()
    }

    /// The per-frame poll (step 81.2, item M97): runs the parked intent
    /// of a completed answer. The shell calls it once per frame at the
    /// top — after the previous frame's paint, before this one; the
    /// focused kittest harnesses call it themselves (they stand in for
    /// the shell).
    pub fn poll_dialogs(&mut self) {
        let Some(answer) = self.dialogs.poll_answer() else {
            return;
        };
        let Some(intent) = self.pending_dialog.take() else {
            return;
        };
        self.run_dialog_intent(answer, intent);
    }

    fn run_dialog_intent(&mut self, answer: DialogAnswer, intent: DialogIntent) {
        match (intent, answer) {
            (intent, DialogAnswer::Unavailable) => self.dialog_unavailable(intent),
            (DialogIntent::FillOpenPath, DialogAnswer::Path(Some(path))) => {
                self.open_path = path.display().to_string();
            }
            (DialogIntent::FillSaveCopyPath, DialogAnswer::Path(Some(path))) => {
                self.save_as_path = path.display().to_string();
            }
            (DialogIntent::FillNewDatabasePath, DialogAnswer::Path(Some(path))) => {
                self.new_database.path = path.display().to_string();
            }
            (DialogIntent::ImportSource, DialogAnswer::Path(Some(path))) => {
                self.set_import_source(path.display().to_string());
                self.dispatch(Command::PreviewImportFile { source: path });
            }
            (DialogIntent::SaveCopy { path, password }, DialogAnswer::Confirmed(true)) => {
                self.dispatch(Command::SaveDatabaseAs {
                    path: path.into(),
                    password,
                });
            }
            (DialogIntent::DeleteRow { table, row }, DialogAnswer::Confirmed(true)) => {
                self.dispatch(Command::DeleteRow { table, row });
            }
            (DialogIntent::StageRowDelete { tab_id, slot }, DialogAnswer::Confirmed(true)) => {
                self.stage_row_delete(tab_id, slot);
            }
            (DialogIntent::DeleteObject { command }, DialogAnswer::Confirmed(true)) => {
                self.dispatch(command);
            }
            (DialogIntent::ExportReportPdf { report }, DialogAnswer::Path(Some(path))) => {
                self.dispatch(Command::ExportReportPdf { report, path });
            }
            // A user's Cancel clears the park and keeps every buffer.
            _ => {}
        }
    }

    /// The host cannot answer at all (no helper thread, a dead dialog
    /// backend): a precise error notice — never a silent pretend-Cancel.
    fn dialog_unavailable(&mut self, _intent: DialogIntent) {
        self.record_error(
            t("The dialog could not be shown — the action was not carried out.").to_owned(),
        );
    }
}
