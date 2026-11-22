//! PDF report export dispatch (phase 97, step 97.2; item M128).

use std::path::Path;

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;

impl Core {
    /// Exports one report's print preview to a new PDF file: the core
    /// paginates once (`report_preview`), the writer renders exactly that
    /// page sequence. The source database and the sidecar stay untouched;
    /// an existing target is refused.
    pub(crate) fn export_report_pdf(&mut self, report: &str, path: &Path) -> Vec<Event> {
        let Some(database) = self.state.database.as_ref() else {
            return failed(FailureCode::NoDatabase, "no database is open");
        };
        let Some(definition) = database
            .reports
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(report))
            .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                format!("report {report:?} not found"),
            );
        };
        let pages = match self.report_preview(&definition.name) {
            Ok(pages) => pages,
            Err(reason) => return failed(FailureCode::Parse, reason),
        };
        match crate::pdf::export_pdf(&definition, &pages, path) {
            Ok(()) => vec![Event::ReportExported {
                report: definition.name,
                path: path.to_path_buf(),
                pages: pages.len(),
            }],
            Err(reason) if reason == crate::pdf::FEATURE_MISSING => {
                failed(FailureCode::Unsupported, reason)
            }
            Err(reason) => failed(FailureCode::Io, reason),
        }
    }
}
