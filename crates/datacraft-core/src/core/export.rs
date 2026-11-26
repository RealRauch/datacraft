//! File export dispatch (phase 97, step 97.2, item M128; phase 99, step
//! 99.1, item M130).

use std::path::Path;

use crate::core::{Core, failed};
use crate::event::Event;
use crate::export::ExportFormat;
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

    /// Exports one table to a new CSV or JSON file (phase 99, step 99.1;
    /// item M130): the whole table in storage order, read through the
    /// port once. The source stays untouched; an existing target is
    /// refused by the writer, and a value JSON cannot carry answers
    /// `Unsupported` instead of writing a partial document.
    pub(crate) fn export_table(
        &mut self,
        table: &str,
        format: ExportFormat,
        path: &Path,
    ) -> Vec<Event> {
        let Some(database) = self.state.database.as_ref() else {
            return failed(FailureCode::NoDatabase, "no database is open");
        };
        let Some(definition) = database
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
            .cloned()
        else {
            return failed(FailureCode::NotFound, format!("table {table:?} not found"));
        };
        let rows = match self.port.read_rows(
            &definition.name,
            crate::port::RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(error) => return failed(FailureCode::Port, error.to_string()),
        };
        match crate::export::export_table(&definition, &rows, format, path) {
            Ok(()) => vec![Event::TableExported {
                table: definition.name,
                format,
                path: path.to_path_buf(),
                rows: rows.len(),
            }],
            Err(error) => {
                let code = match &error {
                    crate::export::ExportError::Unrepresentable(_) => FailureCode::Unsupported,
                    crate::export::ExportError::Shape(_) => FailureCode::Port,
                    crate::export::ExportError::Write(_) => FailureCode::Io,
                };
                failed(code, error.to_string())
            }
        }
    }
}
