//! Datasheet editing flows (steps 11.7/40.4; pending batch step 85.2,
//! item H61) — split from `flows.rs` for the file-size gate (L30).

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;

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
        // `col` is the display position the grid paints; the value lives
        // at the storage column (phase 101, step 101.4; M132).
        let storage = self.storage_column(tab_id, col);
        let staged = self
            .staged_rows(tab_id)
            .and_then(|buffer| buffer.row_image(slot))
            .and_then(|values| values.get(storage))
            .map(crate::labels::value_label);
        let buffer = staged.unwrap_or_else(|| {
            self.rows_for(tab_id)
                .and_then(|rows| rows.get(slot))
                .and_then(|values| values.get(storage))
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
        // The display position maps onto the storage column (phase 101,
        // step 101.4; M132) — identity without a reorder. Read before the
        // buffer borrow below.
        let col = self.storage_column(tab_id, col);
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
}
