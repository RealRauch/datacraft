//! The Import File chain and preview (step 58.1, item M50; moved from
//! `database.rs` for the size gate): one external CSV file becomes a
//! table over the ordinary write path.

use std::path::Path;

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::DriverCapabilities;
use crate::schema::TableDef;

impl Core {
    /// Imports an external CSV file as a new table into the open database
    /// (step 58.1, item M50): parse and infer the source, then create and
    /// fill the table over the ordinary write path — this handler
    /// orchestrates the same `CreateTable`/`InsertRow` handlers every
    /// other client uses, so capability gates, validation, defaults and
    /// rules all apply per row. The source file is never touched. A row
    /// that fails coercion stops the chain with the precise field-named
    /// reason; the rows inserted before it stay (Access-like import
    /// semantics). A full import closes the window and drops the preview.
    pub(crate) fn import_file(&mut self, source: &Path, table: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // The chain needs both write affordances; answer before parsing so
        // a read-only source never reads its import source in vain.
        if !db.capabilities.create_table {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::CreateTables),
                DriverCapabilities::unsupported(&db.driver, "create tables for this source"),
            );
        }
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        if table.trim().is_empty() {
            return failed(FailureCode::InvalidInput, "the target table name is empty");
        }
        let parsed = match crate::import::read_csv_file(source) {
            Ok(parsed) => parsed,
            Err(reason) => {
                return failed(FailureCode::Io, format!("{}: {reason}", source.display()));
            }
        };
        let definition = TableDef::new(table.to_owned(), parsed.fields.clone());
        let mut events = self.create_table(definition);
        if events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
        {
            return events;
        }
        for (index, cells) in parsed.rows.iter().enumerate() {
            match crate::row_edit::row_values(&parsed.fields, cells) {
                Ok(values) => events.extend(self.insert_row(table, &values)),
                Err(reason) => {
                    events.extend(failed(
                        FailureCode::InvalidInput,
                        format!("row {}: {reason}", index + 1),
                    ));
                    return events;
                }
            }
        }
        self.state.import_window_open = false;
        self.state.import_preview = None;
        events
    }

    /// Parses an external CSV file into the Import File window's preview
    /// (step 58.1, item M50): field names, inferred kinds and the leading
    /// rows land in state; the snapshot carries them to every client. The
    /// source is only ever read.
    pub(crate) fn preview_import_file(&mut self, source: &Path) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        }
        let parsed = match crate::import::read_csv_file(source) {
            Ok(parsed) => parsed,
            Err(reason) => {
                self.state.import_preview = None;
                return failed(FailureCode::Io, format!("{}: {reason}", source.display()));
            }
        };
        let table = source.file_stem().map_or_else(
            || "imported".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        let total_rows = parsed.rows.len();
        self.state.import_preview = Some(crate::import::ImportPreview {
            table,
            fields: parsed.fields,
            rows: parsed
                .rows
                .into_iter()
                .take(crate::import::PREVIEW_ROWS)
                .collect(),
            total_rows,
        });
        Vec::new()
    }
}
