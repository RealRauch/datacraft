//! The datasheet's pending-edit batch (step 85.2, item H61).
//!
//! A per-tab [`EditBuffer`](datacraft_core::edit_buffer::EditBuffer)
//! stages updates, inserts and deletes while the datasheet is edited;
//! Ctrl+S previews the journal batch, Apply submits it as one atomic
//! write. The journal is the single write model — the buffer only
//! collects what `PreviewWrite`/`ApplyWrite` apply, and the snapshot's
//! `pending_write` summary is the observable state every client sees.

use datacraft_core::command::Command;
use datacraft_core::edit_buffer::EditBuffer;
use datacraft_core::event::Event;
use datacraft_core::ids::TabId;
use datacraft_core::schema::FieldValue;

use super::App;

/// One tab's open submission: the previewed journal changes between
/// `PreviewWrite` and its apply or discard.
pub(crate) struct Submission {
    /// The tab whose buffer was submitted.
    pub tab: TabId,
    /// The journal's token.
    pub token: u64,
    /// The journal's own diff (`Event::WritePreviewed`) — the preview
    /// renders exactly this, never a second generator.
    pub changes: Vec<datacraft_core::journal::RowChange>,
}

impl App {
    /// The tab's edit buffer, lazily built over the loaded rows (the
    /// grid's live image at buffer start).
    pub fn edit_buffer(&mut self, tab_id: TabId) -> Option<&mut EditBuffer> {
        let rows = self.rows.get(&tab_id)?.clone();
        Some(
            self.edit_buffers
                .entry(tab_id)
                .or_insert_with(|| EditBuffer::new(rows)),
        )
    }

    /// The tab's non-empty edit buffer, read-only (the datasheet renders
    /// its pending image from this).
    pub fn staged_rows(&self, tab_id: TabId) -> Option<&EditBuffer> {
        self.edit_buffers
            .get(&tab_id)
            .filter(|buffer| !buffer.is_empty())
    }

    /// Whether the tab has staged edits (the Ctrl+S branch's guard).
    pub fn has_staged_edits(&self, tab_id: TabId) -> bool {
        self.staged_rows(tab_id).is_some()
    }

    /// Whether the datasheet writes through the pending batch: the
    /// driver applies journal batches atomically (M20). Without the
    /// capability the single-cell path stays as it was (H61).
    pub(crate) fn batch_writes(&self) -> bool {
        self.snapshot
            .database
            .as_ref()
            .is_some_and(|db| db.capabilities.atomic_batch && db.capabilities.write_rows)
    }

    /// Stages one cell update (full positional value list; `slot` is the
    /// buffer row the grid addresses). A refusal reports precisely.
    pub fn stage_cell_update(
        &mut self,
        tab_id: TabId,
        slot: usize,
        values: Vec<FieldValue>,
    ) -> bool {
        let Some(buffer) = self.edit_buffer(tab_id) else {
            return false;
        };
        match buffer.stage_update(slot, values) {
            Ok(()) => true,
            Err(reason) => {
                self.record_error(reason);
                false
            }
        }
    }

    /// Stages a pending insert (the datasheet's trailing new record).
    pub fn stage_row_insert(&mut self, tab_id: TabId, values: Vec<FieldValue>) -> bool {
        let Some(buffer) = self.edit_buffer(tab_id) else {
            return false;
        };
        buffer.stage_insert(values);
        true
    }

    /// Stages a delete of one buffer row; a refusal reports precisely.
    pub fn stage_row_delete(&mut self, tab_id: TabId, slot: usize) -> bool {
        let Some(buffer) = self.edit_buffer(tab_id) else {
            return false;
        };
        match buffer.stage_delete(slot) {
            Ok(()) => true,
            Err(reason) => {
                self.record_error(reason);
                false
            }
        }
    }

    /// Escape discards the staged changes of the active record — only
    /// when no cell editor is armed (Esc cancels that editor first).
    pub(crate) fn discard_staged_record(&mut self, tab_id: TabId) {
        if self.cell_edit.contains_key(&tab_id) {
            return;
        }
        let Some(slot) = self.cursor_slots.get(&tab_id).copied() else {
            return;
        };
        let Some(buffer) = self.edit_buffer(tab_id) else {
            return;
        };
        // A void slot has nothing to revert — silence is correct here.
        if let Err(reason) = buffer.discard_row(slot)
            && !reason.contains("no longer exists")
        {
            self.record_error(reason);
        }
    }

    /// The buffer row (slot) the tab's record cursor addresses, stored by
    /// the datasheet every frame (display indices map through sort and
    /// filter; the Escape path needs the resolved slot).
    pub fn cursor_slot(&self, tab_id: TabId) -> usize {
        self.cursor_slots.get(&tab_id).copied().unwrap_or(0)
    }

    /// Stores the resolved buffer slot of the tab's record cursor.
    pub fn set_cursor_slot(&mut self, tab_id: TabId, slot: usize) {
        self.cursor_slots.insert(tab_id, slot);
    }

    /// Ctrl+S on a table tab with staged edits: previews the journal
    /// batch and opens the submission window. A driver without the
    /// atomic-batch capability or an empty buffer leaves this a no-op
    /// (the shortcut keeps its sidecar meaning).
    pub fn submit_datasheet_batch(&mut self, tab_id: TabId) {
        if !self.batch_writes() {
            return;
        }
        let Some(tab) = self.snapshot.tabs.iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        let table = tab.object_name.clone();
        let ops = self
            .edit_buffers
            .get(&tab_id)
            .filter(|buffer| !buffer.is_empty())
            .map(|buffer| buffer.ops());
        let Some(ops) = ops else {
            return;
        };
        let events = self.dispatch(Command::PreviewWrite { table, ops });
        if let Some(Event::WritePreviewed { token, changes, .. }) = events
            .into_iter()
            .find(|event| matches!(event, Event::WritePreviewed { .. }))
        {
            self.submission = Some(Submission {
                tab: tab_id,
                token,
                changes,
            });
        }
    }

    /// The submission window's Apply: submits the journal token. A
    /// failure keeps the submission open and every edit pending (H61).
    pub fn apply_submission(&mut self) {
        let Some(token) = self.submission.as_ref().map(|submission| submission.token) else {
            return;
        };
        self.dispatch(Command::ApplyWrite { token });
    }

    /// The submission window's Discard (and its close box): drops the
    /// journal so the grid unlocks, keeps the buffer — the user may keep
    /// editing and submit again.
    pub fn discard_submission(&mut self) {
        let Some(token) = self.submission.as_ref().map(|submission| submission.token) else {
            return;
        };
        self.dispatch(Command::DiscardWrite { token });
    }

    /// The open submission, if any (the preview window renders it).
    pub(crate) fn submission(&self) -> Option<&Submission> {
        self.submission.as_ref()
    }

    /// Event follow-through: the buffer clears when its batch applies,
    /// the submission closes on apply/discard, and the tab/database
    /// lifecycle drops the buffers (H48's stale-journal lesson).
    pub(crate) fn maintain_submissions(&mut self, events: &[Event]) {
        if events
            .iter()
            .any(|event| matches!(event, Event::WriteDiscarded { .. }))
        {
            self.submission = None;
        }
        let applied: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                Event::WriteApplied { table, .. } => Some(table.clone()),
                _ => None,
            })
            .collect();
        if !applied.is_empty() {
            self.submission = None;
            let stale: Vec<TabId> = self
                .snapshot
                .tabs
                .iter()
                .filter(|tab| {
                    self.bound_table(tab).is_some_and(|table| {
                        applied
                            .iter()
                            .any(|applied| applied.eq_ignore_ascii_case(&table))
                    })
                })
                .map(|tab| tab.id)
                .collect();
            for tab_id in stale {
                self.edit_buffers.remove(&tab_id);
            }
        }
        // A closed tab's buffer is never addressable again.
        for tab_id in events.iter().filter_map(|event| match event {
            Event::TabClosed { tab_id } => Some(*tab_id),
            _ => None,
        }) {
            self.edit_buffers.remove(&tab_id);
            self.cursor_slots.remove(&tab_id);
        }
        if let Some(submission) = self.submission.as_ref()
            && !self
                .snapshot
                .tabs
                .iter()
                .any(|tab| tab.id == submission.tab)
        {
            self.submission = None;
        }
        if events
            .iter()
            .any(|event| matches!(event, Event::DatabaseClosed))
        {
            self.edit_buffers.clear();
            self.cursor_slots.clear();
            self.submission = None;
        }
    }
}
