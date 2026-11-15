//! Row write handlers (step 62.1 sweep; moved from `database.rs` for the
//! size gate): insert, update and delete run the row preparation first,
//! then the port write, then the event — a rejected row never reaches
//! storage.

use crate::command::CellEdit;
use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::{DriverCapabilities, PortError, RowRange};
use crate::schema::FieldValue;

impl Core {
    /// Appends one row (step 13.2, H36; step 15.2 wires defaults,
    /// calculated fields and rules): row preparation first, then the port
    /// write, then the event — a rejected row never reaches storage.
    pub(crate) fn insert_row(
        &mut self,
        table: &str,
        values: &[crate::schema::FieldValue],
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7): a source the driver
        // declared read-only never reaches the port.
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let canonical = def.name.clone();
        let prepared = match crate::field_expressions::prepare_insert(def, values) {
            Ok(prepared) => prepared,
            Err(reason) => return failed(FailureCode::InvalidInput, reason),
        };
        match self.port.insert_row(&canonical, &prepared) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                self.record_insert(&canonical);
                vec![Event::RowInserted { table: canonical }]
            }
        }
    }

    /// Replaces one row (step 11.10, H40): schema-validated like an insert,
    /// with AutoNumber values preserved (Null keeps the current value, any
    /// other value must match it).
    pub(crate) fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let canonical = def.name.clone();
        let fields = def.fields.clone();
        if values.len() != fields.len() {
            return failed(
                FailureCode::InvalidInput,
                format!(
                    "update {:?}: expected {} values, found {}",
                    canonical,
                    fields.len(),
                    values.len()
                ),
            );
        }
        let current = match self
            .port
            .read_rows(&canonical, crate::port::RowRange { start: row, len: 1 })
        {
            Ok(rows) if rows.len() == 1 => rows.into_iter().next().expect("length checked"),
            Ok(_) => {
                return failed(
                    FailureCode::OutOfRange,
                    format!("row {row} of {canonical:?} is out of range"),
                );
            }
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        let prepared = match crate::field_expressions::prepare_update(def, &current, values) {
            Ok(prepared) => prepared,
            Err(reason) => return failed(FailureCode::InvalidInput, reason),
        };
        match self.port.update_row(&canonical, row, &prepared) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                self.record_update(&canonical, &current, &prepared);
                vec![Event::RowUpdated {
                    table: canonical,
                    row,
                }]
            }
        }
    }

    /// Replaces individual cells across several rows (step 86.3, M101):
    /// every edit is validated against the current images **before the
    /// first write**, and the whole fill records **one** history entry —
    /// one undo restores every row. `Null` clears the cell where the
    /// driver allows it (M24's boundary answers otherwise).
    pub(crate) fn update_cells(&mut self, table: &str, edits: &[CellEdit]) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let canonical = def.name.clone();
        let fields = def.fields.clone();
        if edits.is_empty() {
            return failed(FailureCode::InvalidInput, "no cell edits to apply");
        }
        let rows = match self.port.read_rows(
            &canonical,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        // Validation first: every edit resolves against the current image
        // (row and column in range, the value schema-valid). Edits group
        // per row so a double edit of one row writes and undoes as one
        // consistent image.
        let mut row_order: Vec<usize> = Vec::new();
        let mut per_row: std::collections::BTreeMap<usize, Vec<&CellEdit>> =
            std::collections::BTreeMap::new();
        for edit in edits {
            per_row
                .entry(edit.row)
                .or_insert_with(|| {
                    row_order.push(edit.row);
                    Vec::new()
                })
                .push(edit);
        }
        let mut planned: Vec<(usize, Vec<FieldValue>, Vec<FieldValue>)> = Vec::new();
        for (row, cell_edits) in &per_row {
            let Some(before) = rows.get(*row) else {
                return failed(
                    FailureCode::OutOfRange,
                    format!("row {row} of {canonical:?} is out of range"),
                );
            };
            let mut after = before.clone();
            for edit in cell_edits {
                if edit.col >= fields.len() {
                    return failed(
                        FailureCode::InvalidInput,
                        format!(
                            "column {} is out of range ({} fields)",
                            edit.col,
                            fields.len()
                        ),
                    );
                }
                after[edit.col] = edit.value.clone();
            }
            let prepared = match crate::field_expressions::prepare_update(def, before, &after) {
                Ok(prepared) => prepared,
                Err(reason) => return failed(FailureCode::InvalidInput, reason),
            };
            planned.push((*row, before.clone(), prepared));
        }
        // Apply: one port write per touched row, then the single history
        // entry for the whole fill.
        let mut events = Vec::new();
        let mut images: Vec<(Vec<FieldValue>, Vec<FieldValue>)> = Vec::new();
        for (row, before, after) in &planned {
            if let Err(e) = self.port.update_row(&canonical, *row, after) {
                return failed(FailureCode::Port, e.to_string());
            }
            images.push((before.clone(), after.clone()));
            events.push(Event::RowUpdated {
                table: canonical.clone(),
                row: *row,
            });
        }
        self.record_updates(&canonical, &images);
        events
    }

    /// Deletes one row (step 11.10, H40).
    pub(crate) fn delete_row(&mut self, table: &str, row: usize) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // Capability gate (phase 30, M17; ADR-7).
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return failed(
                FailureCode::NotFound,
                PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let canonical = def.name.clone();
        let current = match self
            .port
            .read_rows(&canonical, crate::port::RowRange { start: row, len: 1 })
        {
            Ok(rows) if rows.len() == 1 => rows.into_iter().next().expect("length checked"),
            Ok(_) => {
                return failed(
                    FailureCode::OutOfRange,
                    format!("row {row} of {canonical:?} is out of range"),
                );
            }
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        match self.port.delete_row(&canonical, row) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                self.record_delete(&canonical, &current);
                vec![Event::RowDeleted {
                    table: canonical,
                    row,
                }]
            }
        }
    }
}
