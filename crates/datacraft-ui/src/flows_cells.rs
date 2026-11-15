//! Datasheet cell-write flows (steps 86.2-86.4, item M101/M102) — split
//! from `flows_datasheet.rs` for the file-size gate: paste, fill, clear
//! to NULL and the value viewer's multiline commit all write through the
//! tab's normal edit path (the H61 batch on an atomic-batch driver, one
//! bus write otherwise), with validation preceding every cell change.

use datacraft_core::command::{CellEdit, Command};
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::schema::FieldValue;
use datacraft_core::selection::Selection;

use crate::app::App;
use crate::i18n::tr;

impl App {
    /// The row's base image for staged or planned edits: the staged image
    /// when the buffer row exists, the cache row otherwise (step 85.2,
    /// item H61).
    fn base_row(&self, tab_id: TabId, slot: usize) -> Option<Vec<FieldValue>> {
        self.staged_rows(tab_id)
            .and_then(|buffer| buffer.row_image(slot))
            .map(<[FieldValue]>::to_vec)
            .or_else(|| {
                self.rows_for(tab_id)
                    .and_then(|rows| rows.get(slot))
                    .cloned()
            })
    }

    /// Paste (step 86.2, item M101): parses the clipboard TSV, validates
    /// the fit onto the tab's selection and every pasted value against
    /// the column kind **before any cell changes**, then stages the
    /// updates (atomic-batch driver) or dispatches one `UpdateRow` per
    /// target row. A rejection reports precisely and leaves the database
    /// and the buffer untouched. `slots` maps display rows onto buffer
    /// rows (sort and filter make them differ).
    pub fn paste_clipboard(&mut self, tab_id: TabId, table: &str, slots: &[usize]) {
        if crate::availability::restricted(
            self.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_some()
        {
            return;
        }
        let Some(text) = self.clipboard_text() else {
            self.record_error("the clipboard is unavailable — nothing to paste".to_owned());
            return;
        };
        let matrix = match datacraft_core::clipboard::parse_tsv(&text) {
            Ok(matrix) => matrix,
            Err(reason) => {
                self.record_error(reason.to_string());
                return;
            }
        };
        let parsed = self.table_definition(table).map(|definition| {
            (
                definition.name.clone(),
                definition.fields.clone(),
                definition.fields.len(),
            )
        });
        let Some((canonical, fields, arity)) = parsed else {
            self.record_error(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return;
        };
        let row_total = slots.len();
        let selection = self.selection(tab_id, row_total, arity);
        let (row_span, col_span) = match datacraft_core::clipboard::paste_targets(
            &selection,
            matrix.len(),
            matrix[0].len(),
            row_total,
            arity,
        ) {
            Ok(spans) => spans,
            Err(reason) => {
                self.record_error(reason.to_string());
                return;
            }
        };
        // Validation first: every pasted cell parses against its column
        // kind and every target row resolves to a base image — one bad
        // cell rejects the whole paste before any write.
        let mut planned: Vec<(usize, Vec<FieldValue>)> = Vec::new();
        for (matrix_row, row) in row_span.clone().enumerate() {
            let slot = slots.get(row).copied().unwrap_or(row);
            // The row's base image: the staged image when the buffer row
            // exists, the cache row otherwise (step 85.2, item H61).
            let base = self
                .staged_rows(tab_id)
                .and_then(|buffer| buffer.row_image(slot))
                .map(<[FieldValue]>::to_vec)
                .or_else(|| {
                    self.rows_for(tab_id)
                        .and_then(|rows| rows.get(slot))
                        .cloned()
                });
            let Some(mut values) = base else {
                self.record_error(format!("row {row} no longer exists in {canonical:?}"));
                return;
            };
            for (matrix_col, col) in col_span.clone().enumerate() {
                let Some(field) = fields.get(col) else {
                    self.record_error(format!(
                        "field index {col} is out of range ({arity} fields)"
                    ));
                    return;
                };
                let cell = matrix
                    .get(matrix_row)
                    .and_then(|cells| cells.get(matrix_col))
                    .map(String::as_str)
                    .unwrap_or_default();
                match datacraft_core::row_edit::parse_field_value(field, cell) {
                    Ok(value) => values[col] = value,
                    Err(reason) => {
                        self.record_error(format!("field {:?}: {reason}", field.name));
                        return;
                    }
                }
            }
            planned.push((slot, values));
        }
        // Apply: staged rows on an atomic-batch driver (Ctrl+S applies
        // the batch), one dispatch per target row otherwise.
        if self.batch_writes() {
            for (slot, values) in planned {
                if !self.stage_cell_update(tab_id, slot, values) {
                    return;
                }
            }
            return;
        }
        for (slot, values) in planned {
            let events = self.dispatch(Command::UpdateRow {
                table: canonical.clone(),
                row: slot,
                values,
            });
            if !events
                .iter()
                .any(|event| matches!(event, Event::RowUpdated { .. }))
            {
                return;
            }
        }
    }

    /// Commits the value viewer's multiline editor (step 86.4, item
    /// M102): the draft parses against the field's kind and writes through
    /// the normal edit path - the H61 batch on an atomic-batch driver, one
    /// `UpdateRow` otherwise. A parse or write failure reports precisely
    /// and keeps the viewer open (`false`).
    pub fn commit_value_editor(
        &mut self,
        tab_id: TabId,
        table: &str,
        row_display: usize,
        slot: usize,
        col: usize,
    ) -> bool {
        let Some(text) = self.value_editor_draft(tab_id) else {
            return false;
        };
        let parsed = self.table_definition(table).map(|definition| {
            (
                definition.name.clone(),
                definition.fields.get(col).map(|field| {
                    datacraft_core::row_edit::parse_field_value(field, &text)
                        .map_err(|reason| format!("field {:?}: {reason}", field.name))
                }),
                definition.fields.len(),
            )
        });
        let Some((canonical, parsed, arity)) = parsed else {
            self.record_error(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return false;
        };
        let value = match parsed {
            Some(Ok(value)) => value,
            Some(Err(reason)) => {
                self.record_error(reason);
                return false;
            }
            None => {
                self.record_error(format!(
                    "field index {col} is out of range ({arity} fields)"
                ));
                return false;
            }
        };
        let Some(mut values) = self.base_row(tab_id, slot) else {
            self.record_error(format!(
                "row {row_display} no longer exists in {canonical:?}"
            ));
            return false;
        };
        if col >= values.len() {
            self.record_error(format!(
                "field index {col} is out of range ({arity} fields)"
            ));
            return false;
        }
        values[col] = value;
        if self.batch_writes() {
            return self.stage_cell_update(tab_id, slot, values);
        }
        let events = self.dispatch(Command::UpdateRow {
            table: canonical,
            row: slot,
            values,
        });
        let ok = events
            .iter()
            .any(|event| matches!(event, Event::RowUpdated { .. }));
        if !ok
            && let Some(reason) = events.iter().find_map(|event| match event {
                Event::Failed { reason, .. } => Some(reason.clone()),
                _ => None,
            })
        {
            self.record_error(reason);
        }
        ok
    }

    /// Fill down (step 86.3, item M101, Ctrl+D): the top row of the
    /// selection fills the rows below it in the same columns with its
    /// values. The fill is one command - staged into the H61 batch on an
    /// atomic-batch driver, one `UpdateCells` write (one undo step)
    /// otherwise.
    pub fn fill_selection(
        &mut self,
        tab_id: TabId,
        table: &str,
        slots: &[usize],
        selection: Selection,
    ) {
        if crate::availability::restricted(
            self.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_some()
        {
            return;
        }
        let parsed = self
            .table_definition(table)
            .map(|definition| (definition.name.clone(), definition.fields.len()));
        let Some((canonical, arity)) = parsed else {
            self.record_error(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return;
        };
        let row_span = selection.row_range(slots.len());
        let col_span = selection.col_range(arity);
        let Some(&anchor_slot) = slots.get(row_span.start) else {
            return;
        };
        let Some(anchor) = self.base_row(tab_id, anchor_slot) else {
            self.record_error(format!(
                "row {} no longer exists in {canonical:?}",
                row_span.start
            ));
            return;
        };
        let mut edits: Vec<CellEdit> = Vec::new();
        let mut planned_rows: Vec<(usize, Vec<FieldValue>)> = Vec::new();
        for row in row_span.clone().skip(1) {
            let slot = slots.get(row).copied().unwrap_or(row);
            let Some(mut values) = self.base_row(tab_id, slot) else {
                self.record_error(format!("row {row} no longer exists in {canonical:?}"));
                return;
            };
            for col in col_span.clone() {
                let Some(value) = anchor.get(col) else {
                    self.record_error(format!("column {col} is out of range ({arity} fields)"));
                    return;
                };
                edits.push(CellEdit {
                    row: slot,
                    col,
                    value: value.clone(),
                });
                values[col] = value.clone();
            }
            planned_rows.push((slot, values));
        }
        if self.batch_writes() {
            for (slot, values) in planned_rows {
                if !self.stage_cell_update(tab_id, slot, values) {
                    return;
                }
            }
            return;
        }
        let events = self.dispatch(Command::UpdateCells {
            table: canonical,
            edits,
        });
        if let Some(reason) = events.iter().find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        }) {
            self.record_error(reason);
        }
    }

    /// Delete-to-NULL (step 86.3, item M101, Del over an extended
    /// selection): every selected cell clears to its empty value - the
    /// parse of the empty text, so the required/zero-length rules and the
    /// driver's NULL boundary (M24) answer precisely. One command, like
    /// the fill.
    pub fn clear_selection(
        &mut self,
        tab_id: TabId,
        table: &str,
        slots: &[usize],
        selection: Selection,
    ) {
        if crate::availability::restricted(
            self.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_some()
        {
            return;
        }
        let parsed = self
            .table_definition(table)
            .map(|definition| (definition.name.clone(), definition.fields.clone()));
        let Some((canonical, fields)) = parsed else {
            self.record_error(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return;
        };
        let arity = fields.len();
        let row_span = selection.row_range(slots.len());
        let col_span = selection.col_range(arity);
        // Validation first: every cell's empty-value parse resolves
        // before anything stages or writes.
        let mut edits: Vec<CellEdit> = Vec::new();
        let mut planned_rows: Vec<(usize, Vec<FieldValue>)> = Vec::new();
        for row in row_span.clone() {
            let slot = slots.get(row).copied().unwrap_or(row);
            let Some(mut values) = self.base_row(tab_id, slot) else {
                self.record_error(format!("row {row} no longer exists in {canonical:?}"));
                return;
            };
            for col in col_span.clone() {
                let Some(field) = fields.get(col) else {
                    self.record_error(format!("column {col} is out of range ({arity} fields)"));
                    return;
                };
                let value = match datacraft_core::row_edit::parse_field_value(field, "") {
                    Ok(value) => value,
                    Err(reason) => {
                        self.record_error(format!("field {:?}: {reason}", field.name));
                        return;
                    }
                };
                edits.push(CellEdit {
                    row: slot,
                    col,
                    value: value.clone(),
                });
                values[col] = value;
            }
            planned_rows.push((slot, values));
        }
        if self.batch_writes() {
            for (slot, values) in planned_rows {
                if !self.stage_cell_update(tab_id, slot, values) {
                    return;
                }
            }
            return;
        }
        let events = self.dispatch(Command::UpdateCells {
            table: canonical,
            edits,
        });
        if let Some(reason) = events.iter().find_map(|event| match event {
            Event::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        }) {
            self.record_error(reason);
        }
    }
}
