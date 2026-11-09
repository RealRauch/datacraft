//! Form runtime edit buffer (phase 64, item M68; ADR-8).
//!
//! Typing edits a per-record buffer; exactly one `UpdateRow` is dispatched
//! when the record is left. `Esc` reverts the current field first, the
//! whole record on the second press. The state lives in one `App` field
//! (the size gate; the shell struct itself stays lean).

use std::collections::{BTreeMap, HashMap, HashSet};

use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
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
