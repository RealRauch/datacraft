//! Datasheet editing flows (steps 11.7/40.4; pending batch step 85.2,
//! item H61) — split from `flows.rs` for the file-size gate (L30).

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::schema::FieldValue;

use crate::app::App;
use crate::i18n::tr;

impl App {
    /// Inserts the buffered new-record row of a datasheet tab (step 11.7).
    /// Cell parsing errors land in the status bar; a successful insert
    /// clears the buffer so the next record starts empty. On an
    /// atomic-batch driver the record stages into the edit buffer (step
    /// 85.2, item H61) instead of dispatching.
    pub fn insert_new_row(&mut self, tab_id: TabId, table: &str) -> bool {
        let parsed = self.table_definition(table).map(|definition| {
            let cells = self.new_row_cells(tab_id, definition.fields.len());
            (
                definition.name.clone(),
                datacraft_core::row_edit::row_values(&definition.fields, &cells),
            )
        });
        let Some((canonical, parsed)) = parsed else {
            self.record_error(tr(
                "table {table} not found",
                &[("table", &format!("{table:?}"))],
            ));
            return false;
        };
        let values = match parsed {
            Ok(values) => values,
            Err(reason) => {
                self.record_error(reason);
                return false;
            }
        };
        if self.batch_writes() {
            if self.stage_row_insert(tab_id, values) {
                self.clear_new_row(tab_id);
                true
            } else {
                false
            }
        } else {
            let events = self.dispatch(Command::InsertRow {
                table: canonical,
                values,
            });
            if events
                .iter()
                .any(|event| matches!(event, Event::RowInserted { .. }))
            {
                self.clear_new_row(tab_id);
                true
            } else {
                false
            }
        }
    }

    /// Arms the datasheet's in-place cell editor on one cell of a table
    /// tab with the cell's current display text (step 40.4, item L23).
    /// A no-op for restricted sources — the grid never arms there. With
    /// staged edits the cell seeds from the buffer's pending image (step
    /// 85.2, item H61); `row` is the display row the grid paints, `slot`
    /// the buffer row it addresses.
    pub fn start_cell_edit(&mut self, tab_id: TabId, row: usize, slot: usize, col: usize) {
        if crate::availability::restricted(
            self.snapshot(),
            datacraft_core::availability::ids::EDIT_RECORD,
        )
        .is_some()
        {
            return;
        }
        let staged = self
            .staged_rows(tab_id)
            .and_then(|buffer| buffer.row_image(slot))
            .and_then(|values| values.get(col))
            .map(crate::labels::value_label);
        let buffer = staged.unwrap_or_else(|| {
            self.rows_for(tab_id)
                .and_then(|rows| rows.get(slot))
                .and_then(|values| values.get(col))
                .map(crate::labels::value_label)
                .unwrap_or_default()
        });
        self.arm_cell_edit(tab_id, row, col, buffer);
    }

    /// Commits the armed in-place editor (step 40.4, item L23): the
    /// buffer is parsed against the field's kind. On an atomic-batch
    /// driver the edit stages into the tab's edit buffer (step 85.2,
    /// item H61); otherwise exactly one `UpdateRow` dispatch writes the
    /// full positional value list. A parse failure keeps the editor
    /// armed with the buffer intact and reports the precise reason; a
    /// failed staging reports and closes the editor.
    pub fn commit_cell_edit(&mut self, tab_id: TabId, table: &str, row: usize, col: usize) {
        let Some((_, _, buffer)) = self.cell_edit_parts(tab_id) else {
            return;
        };
        let text = buffer.clone();
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
            return;
        };
        let value = match parsed {
            Some(Ok(value)) => value,
            Some(Err(reason)) => {
                self.record_error(reason);
                return;
            }
            None => {
                self.record_error(format!(
                    "field index {col} is out of range ({arity} fields)"
                ));
                return;
            }
        };
        // The row's base image: the staged image when the buffer row
        // exists, the cache row otherwise (step 85.2, item H61).
        let base = self
            .staged_rows(tab_id)
            .and_then(|buffer| buffer.row_image(row))
            .map(<[datacraft_core::schema::FieldValue]>::to_vec)
            .or_else(|| {
                self.rows_for(tab_id)
                    .and_then(|rows| rows.get(row))
                    .cloned()
            });
        let Some(mut values) = base else {
            self.record_error(format!("row {row} no longer exists in {canonical:?}"));
            return;
        };
        if col >= values.len() {
            self.record_error(format!(
                "field index {col} is out of range ({arity} fields)"
            ));
            return;
        }
        values[col] = value;
        if self.batch_writes() {
            if self.stage_cell_update(tab_id, row, values) {
                self.close_cell_edit(tab_id);
            }
            return;
        }
        let events = self.dispatch(Command::UpdateRow {
            table: canonical,
            row,
            values,
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::RowUpdated { .. }))
        {
            self.close_cell_edit(tab_id);
        }
    }

    /// Closes the armed in-place editor without committing (Esc / the
    /// grid's cancel intent).
    pub fn cancel_cell_edit(&mut self, tab_id: TabId) {
        self.close_cell_edit(tab_id);
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
}
