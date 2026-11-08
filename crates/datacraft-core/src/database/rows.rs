//! Row write handlers (step 62.1 sweep; moved from `database.rs` for the
//! size gate): insert, update and delete run the row preparation first,
//! then the port write, then the event — a rejected row never reaches
//! storage.

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::{DriverCapabilities, PortError};
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
