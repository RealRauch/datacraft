//! Datasheet edit buffer (step 85.1, item H61).
//!
//! A per-tab staging area for datasheet edits: updates, inserts and
//! deletes accumulate against a **stable slot list** — one slot per row
//! as the grid shows it, never reordered, so grid indices and buffer
//! indices stay identical while edits are pending. Deleted rows stay
//! visible (the delete marker keeps their image), discarded inserts
//! leave a [`RowState::Void`] slot instead of shifting everything behind
//! them. Serializing through [`EditBuffer::ops`] yields the journal's
//! [`WriteOp`] batch with the **evolving indices**
//! ([`crate::journal`]) — the buffer has no second write model, it only
//! collects what the journal applies as one atomic batch.

use serde::{Deserialize, Serialize};

use crate::journal::WriteOp;
use crate::schema::FieldValue;

/// Pending state of one grid row as the datasheet renders it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowState {
    /// The row is unmodified.
    Unchanged,
    /// The row carries pending updates (amber markers on the changed cells).
    Updated,
    /// The row carries a pending delete (marker keeps the row visible).
    Deleted,
    /// The row is a pending insert (the trailing new record).
    Inserted,
    /// Nothing: a discarded insert's slot (or an out-of-range row).
    Void,
}

/// One slot of the buffer. `base` is the row image at buffer start —
/// the revert target of [`EditBuffer::discard_row`].
#[derive(Debug, Clone, PartialEq)]
enum Slot {
    /// An existing row with optional pending changes.
    Row {
        base: Vec<FieldValue>,
        pending: Pending,
    },
    /// A pending insert; [`EditBuffer::stage_update`] edits its values in
    /// place, [`EditBuffer::discard_row`] voids the slot.
    Insert { values: Vec<FieldValue> },
    /// A discarded insert. The slot stays so the indices of everything
    /// behind it never shift mid-buffer.
    Void,
}

/// Pending change of one existing row.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    /// No staged change.
    None,
    /// The row's pending value image (full positional list).
    Update(Vec<FieldValue>),
    /// The row is staged for deletion.
    Delete,
}

/// The per-tab edit buffer of one datasheet (one table per tab).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EditBuffer {
    slots: Vec<Slot>,
}

impl EditBuffer {
    /// Builds the buffer over the table's loaded rows (the grid's live
    /// image at buffer start).
    pub fn new(rows: Vec<Vec<FieldValue>>) -> Self {
        Self {
            slots: rows
                .into_iter()
                .map(|base| Slot::Row {
                    base,
                    pending: Pending::None,
                })
                .collect(),
        }
    }

    /// Rows the grid renders: live rows, pending inserts, and nothing for
    /// void slots.
    pub fn row_count(&self) -> usize {
        self.slots.len()
    }

    /// True when nothing is staged (every slot unchanged, no insert).
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|slot| match slot {
            Slot::Row { pending, .. } => matches!(pending, Pending::None),
            Slot::Void => true,
            Slot::Insert { .. } => false,
        })
    }

    /// Pending state of one grid row; [`RowState::Void`] for a discarded
    /// insert's slot and for out-of-range rows.
    pub fn row_state(&self, row: usize) -> RowState {
        match self.slots.get(row) {
            Some(Slot::Row {
                pending: Pending::None,
                ..
            }) => RowState::Unchanged,
            Some(Slot::Row {
                pending: Pending::Update(_),
                ..
            }) => RowState::Updated,
            Some(Slot::Row {
                pending: Pending::Delete,
                ..
            }) => RowState::Deleted,
            Some(Slot::Insert { .. }) => RowState::Inserted,
            None | Some(Slot::Void) => RowState::Void,
        }
    }

    /// The row image the grid renders: the pending values of an update,
    /// the original values otherwise (`None` for void slots).
    pub fn row_image(&self, row: usize) -> Option<&[FieldValue]> {
        match self.slots.get(row) {
            Some(Slot::Row {
                pending: Pending::Update(values),
                ..
            }) => Some(values.as_slice()),
            Some(Slot::Row { base, .. }) => Some(base.as_slice()),
            Some(Slot::Insert { values }) => Some(values.as_slice()),
            None | Some(Slot::Void) => None,
        }
    }

    /// Column indices where the row's rendered image differs from its
    /// image at buffer start — the amber-marker positions. Every column
    /// of a pending insert counts.
    pub fn changed_cells(&self, row: usize) -> Vec<usize> {
        match self.slots.get(row) {
            Some(Slot::Row {
                base,
                pending: Pending::Update(values),
            }) => (0..values.len().min(base.len()))
                .filter(|&col| values[col] != base[col])
                .collect(),
            Some(Slot::Insert { values }) => (0..values.len()).collect(),
            _ => Vec::new(),
        }
    }

    fn slot_mut(&mut self, row: usize) -> Result<&mut Slot, String> {
        if row >= self.slots.len() {
            return Err(format!(
                "row {row} is out of range ({} rows in the edit buffer)",
                self.slots.len()
            ));
        }
        Ok(&mut self.slots[row])
    }

    /// Stages an update of one row (full positional value list; the grid
    /// row index addresses the stable slot list). Editing a deleted row
    /// is refused precisely; editing a pending insert replaces its values.
    pub fn stage_update(&mut self, row: usize, values: Vec<FieldValue>) -> Result<(), String> {
        match self.slot_mut(row)? {
            Slot::Row { pending, .. } => {
                if matches!(pending, Pending::Delete) {
                    return Err(format!(
                        "row {row} has a pending delete — discard it first to edit it again"
                    ));
                }
                *pending = Pending::Update(values);
                Ok(())
            }
            Slot::Insert { values: staged } => {
                *staged = values;
                Ok(())
            }
            Slot::Void => Err(format!("row {row} no longer exists in the edit buffer")),
        }
    }

    /// Stages a delete of one row. Deleting a pending insert voids its
    /// slot instead (the record was never in the database).
    pub fn stage_delete(&mut self, row: usize) -> Result<(), String> {
        match self.slot_mut(row)? {
            Slot::Row { pending, .. } => {
                if matches!(pending, Pending::Delete) {
                    return Err(format!(
                        "row {row} has a pending delete — discard it first to edit it again"
                    ));
                }
                *pending = Pending::Delete;
                Ok(())
            }
            Slot::Insert { .. } => {
                self.slots[row] = Slot::Void;
                Ok(())
            }
            Slot::Void => Err(format!("row {row} no longer exists in the edit buffer")),
        }
    }

    /// Stages a pending insert (appended after the last row, the
    /// datasheet's trailing new record).
    pub fn stage_insert(&mut self, values: Vec<FieldValue>) {
        self.slots.push(Slot::Insert { values });
    }

    /// Reverts one row to its image at buffer start (the Escape path):
    /// a pending update or delete is undone, a pending insert is voided,
    /// a void slot stays void.
    pub fn discard_row(&mut self, row: usize) -> Result<(), String> {
        match self.slot_mut(row)? {
            Slot::Row { pending, .. } => {
                *pending = Pending::None;
                Ok(())
            }
            Slot::Insert { .. } => {
                self.slots[row] = Slot::Void;
                Ok(())
            }
            Slot::Void => Err(format!("row {row} no longer exists in the edit buffer")),
        }
    }

    /// Serializes the staged edits onto the journal's batch vocabulary
    /// with the evolving-index contract: surviving rows count up, deletes
    /// shift the indices behind them, inserts append at the end.
    pub fn ops(&self) -> Vec<WriteOp> {
        let mut ops = Vec::new();
        let mut out = 0usize;
        for slot in &self.slots {
            match slot {
                Slot::Void => {}
                Slot::Row {
                    pending: Pending::None,
                    ..
                } => out += 1,
                Slot::Row {
                    pending: Pending::Update(values),
                    ..
                } => {
                    ops.push(WriteOp::Update {
                        row: out,
                        values: values.clone(),
                    });
                    out += 1;
                }
                Slot::Row {
                    pending: Pending::Delete,
                    ..
                } => ops.push(WriteOp::Delete { row: out }),
                Slot::Insert { values } => {
                    ops.push(WriteOp::Insert {
                        values: values.clone(),
                    });
                }
            }
        }
        ops
    }
}
