//! Form runtime edit buffer (phase 64, item M68; ADR-8).
//!
//! Typing edits a per-record buffer; exactly one `UpdateRow` is dispatched
//! when the record is left. `Esc` reverts the current field first, the
//! whole record on the second press. The state lives in one `App` field
//! (the size gate; the shell struct itself stays lean).

use std::collections::{BTreeMap, HashMap, HashSet};

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::ids::TabId;
use datacraft_core::port::RowRange;
use datacraft_core::schema::FieldValue;

use super::App;

/// Pending changes of one form record: the physical row plus the changed
/// columns (typed values, ready for the single `UpdateRow`).
#[derive(Debug, Default)]
struct PendingRecord {
    row: usize,
    values: BTreeMap<usize, FieldValue>,
}

/// Runtime edit state of every form tab.
#[derive(Debug, Default)]
pub(crate) struct FormEditState {
    pending: HashMap<TabId, PendingRecord>,
    drafts: HashMap<(TabId, usize), String>,
    esc_armed: HashSet<TabId>,
    /// Cached display values of combo row sources per (tab, control).
    combo_cache: HashMap<(TabId, String), Vec<FieldValue>>,
    /// Buffered row-source/display-field pair of the designer's combo
    /// panel, keyed by tab and control (phase 64, M70).
    combo_drafts: HashMap<(TabId, String), (String, String)>,
    /// Row indices of a filtered form record set (phase 64, M71). Absent
    /// means the filter is off or not computed yet.
    filtered: HashMap<TabId, Vec<usize>>,
    /// Precise failure of the last filter evaluation, if any.
    filter_errors: HashMap<TabId, String>,
    /// Physical row the runtime showed last, per form tab: keeps the
    /// cursor on the same record when a filter changes (Access keeps the
    /// current record visible instead of the ordinal).
    last_physical: HashMap<TabId, usize>,
    /// Last row-index set per form tab: the cursor remap only fires when
    /// the record set itself changed, never on ordinary navigation.
    last_indices: HashMap<TabId, Vec<usize>>,
}

impl App {
    /// The staged value of one column for the current record, if any.
    pub fn form_pending_value(&self, tab_id: TabId, col: usize) -> Option<&FieldValue> {
        self.form_edit
            .pending
            .get(&tab_id)
            .and_then(|pending| pending.values.get(&col))
    }

    /// Whether the tab holds unsaved record edits.
    pub fn form_has_pending(&self, tab_id: TabId) -> bool {
        self.form_edit
            .pending
            .get(&tab_id)
            .is_some_and(|pending| !pending.values.is_empty())
    }

    /// The typed draft of one control (present while the user edits).
    pub fn form_draft(&self, tab_id: TabId, col: usize) -> Option<&str> {
        self.form_edit
            .drafts
            .get(&(tab_id, col))
            .map(String::as_str)
    }

    /// Stores the typed draft of one control.
    pub fn set_form_draft(&mut self, tab_id: TabId, col: usize, text: String) {
        self.form_edit.drafts.insert((tab_id, col), text);
    }

    /// Drops the typed draft of one control.
    pub fn clear_form_draft(&mut self, tab_id: TabId, col: usize) {
        self.form_edit.drafts.remove(&(tab_id, col));
    }

    /// Every display value of a combo's row source, in source order and
    /// including duplicates (Access does not dedupe a row source — ADR-8).
    /// An empty source or a missing field yields an empty list.
    pub fn combo_values(&mut self, tab_id: TabId, control: &ControlDef) -> Vec<FieldValue> {
        let ControlKind::ComboBox {
            row_source,
            display_field,
            ..
        } = &control.kind
        else {
            return Vec::new();
        };
        if row_source.trim().is_empty() || display_field.trim().is_empty() {
            return Vec::new();
        }
        let key = (tab_id, control.name.clone());
        if let Some(values) = self.form_edit.combo_cache.get(&key) {
            return values.clone();
        }
        let values = self.read_combo_values(row_source, display_field);
        self.form_edit.combo_cache.insert(key, values.clone());
        values
    }

    fn read_combo_values(&self, source: &str, field: &str) -> Vec<FieldValue> {
        if let Some(table) = self.table_definition(source) {
            let Some(col) = table
                .fields
                .iter()
                .position(|column| column.name.eq_ignore_ascii_case(field))
            else {
                return Vec::new();
            };
            let table_name = table.name.clone();
            let Ok(rows) = self.core.read_rows(
                &table_name,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            ) else {
                return Vec::new();
            };
            return rows
                .iter()
                .filter_map(|row| row.get(col).cloned())
                .collect();
        }
        let Ok(result) = self.core.run_query(source) else {
            return Vec::new();
        };
        let Some(col) = result
            .columns
            .iter()
            .position(|column| column.eq_ignore_ascii_case(field))
        else {
            return Vec::new();
        };
        result
            .rows
            .iter()
            .filter_map(|row| row.get(col).cloned())
            .collect()
    }

    /// Drops the combo row-source cache (every dispatch may change rows).
    pub(crate) fn invalidate_combo_cache(&mut self) {
        self.form_edit.combo_cache.clear();
    }

    /// The designer's combo row-source draft, seeded from the control.
    pub fn combo_source_draft(
        &self,
        tab_id: TabId,
        control: &str,
        row_source: &str,
        display_field: &str,
    ) -> (String, String) {
        self.form_edit
            .combo_drafts
            .get(&(tab_id, control.to_owned()))
            .cloned()
            .unwrap_or_else(|| (row_source.to_owned(), display_field.to_owned()))
    }

    /// Stores the designer's combo row-source draft.
    pub fn set_combo_source_draft(
        &mut self,
        tab_id: TabId,
        control: &str,
        row_source: String,
        display_field: String,
    ) {
        self.form_edit
            .combo_drafts
            .insert((tab_id, control.to_owned()), (row_source, display_field));
    }

    /// Drops the designer's combo row-source draft after an apply.
    pub fn clear_combo_source_draft(&mut self, tab_id: TabId, control: &str) {
        self.form_edit
            .combo_drafts
            .remove(&(tab_id, control.to_owned()));
    }

    /// The form's own filter criteria, if one is applied (ADR-8).
    pub fn form_filter(&self, form: &str) -> Option<&str> {
        self.core.form_filter(form)
    }

    /// The row indices of the form's filtered record set (absent = no
    /// filter or not computed).
    pub fn form_row_indices(&self, tab_id: TabId) -> Option<&[usize]> {
        self.form_edit.filtered.get(&tab_id).map(Vec::as_slice)
    }

    /// The precise filter-evaluation failure of a form tab, if any.
    pub fn form_filter_error(&self, tab_id: TabId) -> Option<&str> {
        self.form_edit
            .filter_errors
            .get(&tab_id)
            .map(String::as_str)
    }

    /// Stores the filtered row indices and clears the failure.
    pub(crate) fn set_form_filter_rows(&mut self, tab_id: TabId, indices: Vec<usize>) {
        self.form_edit.filtered.insert(tab_id, indices);
        self.form_edit.filter_errors.remove(&tab_id);
    }

    /// Stores a filter-evaluation failure (no indices are applied).
    pub(crate) fn set_form_filter_error(&mut self, tab_id: TabId, reason: String) {
        self.form_edit.filtered.remove(&tab_id);
        self.form_edit.filter_errors.insert(tab_id, reason);
    }

    /// Clears the filter cache of a form tab.
    pub(crate) fn clear_form_filter_rows(&mut self, tab_id: TabId) {
        self.form_edit.filtered.remove(&tab_id);
        self.form_edit.filter_errors.remove(&tab_id);
    }

    /// The record cursor for the current record set: keeps the physical
    /// record when the filter changes and it is still visible, otherwise
    /// clamps the stored ordinal.
    pub fn form_cursor(&mut self, tab_id: TabId, indices: &[usize], row_count: usize) -> usize {
        let stored = self.form_record(tab_id).min(row_count - 1);
        let changed = self
            .form_edit
            .last_indices
            .get(&tab_id)
            .map(|previous| previous.as_slice() != indices)
            .unwrap_or(true);
        let position = if changed {
            self.form_edit
                .last_physical
                .get(&tab_id)
                .copied()
                .and_then(|physical| indices.iter().position(|index| *index == physical))
                .unwrap_or(stored)
        } else {
            stored
        };
        let physical = indices[position];
        self.form_edit.last_physical.insert(tab_id, physical);
        self.form_edit.last_indices.insert(tab_id, indices.to_vec());
        if position != stored {
            self.set_form_record(tab_id, position);
        }
        position
    }

    /// Whether the first `Esc` already reverted a field of this record.
    pub fn form_esc_armed(&self, tab_id: TabId) -> bool {
        self.form_edit.esc_armed.contains(&tab_id)
    }

    /// Stages one typed value for the current record (no dispatch yet).
    pub fn stage_form_value(&mut self, tab_id: TabId, row: usize, col: usize, value: FieldValue) {
        let pending = self.form_edit.pending.entry(tab_id).or_default();
        if pending.row != row {
            pending.row = row;
            pending.values.clear();
        }
        pending.values.insert(col, value);
        self.form_edit.esc_armed.remove(&tab_id);
    }

    /// `Esc` once: reverts the current field and arms the record undo.
    pub fn revert_form_field(&mut self, tab_id: TabId, col: usize) {
        let emptied = {
            let Some(pending) = self.form_edit.pending.get_mut(&tab_id) else {
                return;
            };
            pending.values.remove(&col);
            pending.values.is_empty()
        };
        if emptied {
            self.form_edit.pending.remove(&tab_id);
        }
        self.form_edit.drafts.remove(&(tab_id, col));
        self.form_edit.esc_armed.insert(tab_id);
    }

    /// `Esc` twice: reverts every staged change of the record.
    pub fn revert_form_record(&mut self, tab_id: TabId) {
        self.form_edit.pending.remove(&tab_id);
        self.form_edit.drafts.retain(|(tab, _), _| *tab != tab_id);
        self.form_edit.esc_armed.remove(&tab_id);
    }

    /// Close guard of the dispatch path: flushes the affected form
    /// records before a tab or the database closes. `false` aborts the
    /// command with the buffer (and the precise failure) intact.
    pub(crate) fn guard_close(&mut self, command: &Command) -> bool {
        match command {
            Command::CloseTab { tab_id } => self.flush_form_edits(*tab_id),
            Command::CloseOtherTabs { .. } | Command::CloseAllTabs | Command::CloseDatabase => {
                self.flush_all_form_edits()
            }
            _ => true,
        }
    }

    /// Dispatches every pending form record (tab/database close). Returns
    /// `false` as soon as one write fails.
    pub fn flush_all_form_edits(&mut self) -> bool {
        let tabs: Vec<TabId> = self
            .snapshot
            .tabs
            .iter()
            .filter(|tab| tab.object_kind == "form")
            .map(|tab| tab.id)
            .collect();
        for tab_id in tabs {
            if self.form_has_pending(tab_id) && !self.flush_form_edits(tab_id) {
                return false;
            }
        }
        true
    }

    /// Dispatches the staged record as exactly one `UpdateRow`. Returns
    /// `false` when nothing was written (the caller must not navigate
    /// away or close the tab then).
    pub fn flush_form_edits(&mut self, tab_id: TabId) -> bool {
        let Some(pending) = self.form_edit.pending.get(&tab_id) else {
            return true;
        };
        if pending.values.is_empty() {
            self.revert_form_record(tab_id);
            return true;
        }
        let (row, staged) = (pending.row, pending.values.clone());
        let Some(tab) = self
            .snapshot
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .cloned()
        else {
            return true;
        };
        if tab.object_kind != "form" {
            return true;
        }
        let Some(form) = self
            .snapshot
            .database
            .as_ref()
            .and_then(|db| db.forms.iter().find(|form| form.name == tab.object_name))
            .cloned()
        else {
            return true;
        };
        let Some(cache) = self.rows.get(&tab_id) else {
            self.last_error = Some(format!(
                "form {:?}: the source rows are not loaded",
                form.name
            ));
            return false;
        };
        let Some(base) = cache.get(row) else {
            self.last_error = Some(format!("row {row} no longer exists in {:?}", form.source));
            return false;
        };
        let mut image = base.clone();
        for (col, value) in &staged {
            if *col >= image.len() {
                self.last_error = Some(format!(
                    "field index {col} is out of range ({} fields)",
                    image.len()
                ));
                return false;
            }
            image[*col] = value.clone();
        }
        let table = self
            .snapshot
            .database
            .as_ref()
            .and_then(|db| {
                db.tables
                    .iter()
                    .find(|table| table.name.eq_ignore_ascii_case(&form.source))
            })
            .map(|table| table.name.clone())
            .unwrap_or_else(|| form.source.clone());
        let events = self.dispatch(Command::UpdateRow {
            table,
            row,
            values: image,
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::RowUpdated { .. }))
        {
            self.revert_form_record(tab_id);
            true
        } else {
            false
        }
    }
}
