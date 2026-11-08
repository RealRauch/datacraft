//! Journaled write profile (step 11.11, item M2).
//!
//! A write batch is previewed as a diff, applied only after the caller
//! confirms the token, and carries the inverse operations for a rollback.
//! The profile restricts the batch to qualified field families — anything
//! else is refused precisely instead of being guessed (BACKLOG M2).
//! Backends apply the batch through
//! [`StoragePort::write_row_ops`](crate::port::StoragePort::write_row_ops)
//! ([`crate::port::StoragePort`]), which is atomic on the Jet backend.

use serde::{Deserialize, Serialize};

use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::{DriverCapabilities, RowRange, RowWriteOp};
use crate::schema::{AutoKind, BinaryKind, FieldKind, FieldValue, TableDef};

/// One row operation of a journaled batch. Indices address the row set as
/// it evolves through the batch (sequential application).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WriteOp {
    /// Appends a row (`Null` on an AutoNumber field = engine-assigned).
    Insert {
        /// Full positional value list.
        values: Vec<FieldValue>,
    },
    /// Replaces the row at `row`.
    Update {
        /// Zero-based live-row index in the evolving row set.
        row: usize,
        /// Full positional value list.
        values: Vec<FieldValue>,
    },
    /// Removes the row at `row`.
    Delete {
        /// Zero-based live-row index in the evolving row set.
        row: usize,
    },
}

/// Kind of one previewed change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A row is appended.
    Insert,
    /// A row is replaced.
    Update,
    /// A row is removed.
    Delete,
}

/// One diff entry of a previewed batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowChange {
    /// Operation kind.
    pub kind: ChangeKind,
    /// Row index (insert position, update/delete target).
    pub row: Option<usize>,
    /// Values before the change (update/delete).
    pub before: Option<Vec<FieldValue>>,
    /// Values after the change (insert/update).
    pub after: Option<Vec<FieldValue>>,
}

/// Qualification of a table for journaled writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteProfile {
    /// True when every column is fixed-width (updates stay in place).
    pub fixed_width: bool,
}

/// Journal stored between preview and apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingWrite {
    /// Canonical table name.
    pub table: String,
    /// Confirmation token of this journal.
    pub token: u64,
    /// The previewed operations.
    pub ops: Vec<WriteOp>,
    /// The previewed diff (apply requires it to still match).
    pub changes: Vec<RowChange>,
}

/// Applied journal kept for rollback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedWrite {
    /// Canonical table name.
    pub table: String,
    /// Token of the applied journal.
    pub token: u64,
    /// Inverse operations, in reverse application order.
    pub rollback: Vec<WriteOp>,
}

/// The table's write profile: qualified families only, with the precise
/// reason for every refusal.
pub fn write_profile(table: &TableDef) -> Result<WriteProfile, String> {
    for field in &table.fields {
        match &field.kind {
            FieldKind::Binary(BinaryKind::Attachment) => {
                return Err(format!(
                    "table {:?}: column {:?} (attachment) is outside the write profile — \
                     complex values are not qualified yet (BACKLOG M4)",
                    table.name, field.name
                ));
            }
            FieldKind::AutoNumber(AutoKind::ReplicationId) => {
                return Err(format!(
                    "table {:?}: column {:?} (GUID AutoNumber) is outside the write profile — \
                     the counter is not maintained yet (BACKLOG M8)",
                    table.name, field.name
                ));
            }
            _ => {}
        }
    }
    Ok(WriteProfile {
        fixed_width: table.fields.iter().all(|field| is_fixed_width(&field.kind)),
    })
}

fn is_fixed_width(kind: &FieldKind) -> bool {
    !matches!(
        kind,
        FieldKind::Text { .. } | FieldKind::Memo | FieldKind::Binary(_)
    )
}

/// Simulates the batch over `rows` and returns the **prepared** operations
/// (defaults filled, calculated computed, rules enforced) plus the diff.
/// Read-only; the caller applies only after confirmation — the prepared
/// batch is the contract, so apply writes exactly what the preview showed.
pub fn preview(
    table: &TableDef,
    rows: &[Vec<FieldValue>],
    ops: &[WriteOp],
) -> Result<(Vec<WriteOp>, Vec<RowChange>), String> {
    write_profile(table)?;
    let mut state = rows.to_vec();
    let mut prepared = Vec::with_capacity(ops.len());
    let mut changes = Vec::with_capacity(ops.len());
    for op in ops {
        match op {
            WriteOp::Insert { values } => {
                let values = crate::field_expressions::prepare_insert(table, values)?;
                let row = state.len();
                state.push(values.clone());
                changes.push(RowChange {
                    kind: ChangeKind::Insert,
                    row: Some(row),
                    before: None,
                    after: Some(values.clone()),
                });
                prepared.push(WriteOp::Insert { values });
            }
            WriteOp::Update { row, values } => {
                let Some(current) = state.get(*row) else {
                    return Err(out_of_range(table, *row, state.len()));
                };
                let values = crate::field_expressions::prepare_update(table, current, values)?;
                let before = state[*row].clone();
                state[*row] = values.clone();
                changes.push(RowChange {
                    kind: ChangeKind::Update,
                    row: Some(*row),
                    before: Some(before),
                    after: Some(values.clone()),
                });
                prepared.push(WriteOp::Update { row: *row, values });
            }
            WriteOp::Delete { row } => {
                if *row >= state.len() {
                    return Err(out_of_range(table, *row, state.len()));
                }
                let before = state.remove(*row);
                changes.push(RowChange {
                    kind: ChangeKind::Delete,
                    row: Some(*row),
                    before: Some(before),
                    after: None,
                });
                prepared.push(op.clone());
            }
        }
    }
    Ok((prepared, changes))
}

/// The inverse operations of a previewed diff, in reverse application
/// order. Deletes are undone by appending the before image (the port has
/// no insert-at-index), so a rolled-back delete returns at the end.
pub fn rollback_ops(changes: &[RowChange]) -> Vec<WriteOp> {
    changes
        .iter()
        .rev()
        .map(|change| match change.kind {
            ChangeKind::Insert => WriteOp::Delete {
                row: change.row.expect("inserts carry their index"),
            },
            ChangeKind::Update => WriteOp::Update {
                row: change.row.expect("updates carry their index"),
                values: change
                    .before
                    .clone()
                    .expect("updates carry the before image"),
            },
            ChangeKind::Delete => WriteOp::Insert {
                values: change
                    .before
                    .clone()
                    .expect("deletes carry the before image"),
            },
        })
        .collect()
}

/// Maps journal operations onto the port's batch vocabulary.
pub fn to_row_write_ops(ops: &[WriteOp]) -> Vec<RowWriteOp> {
    ops.iter()
        .map(|op| match op {
            WriteOp::Insert { values } => RowWriteOp::Insert(values.clone()),
            WriteOp::Update { row, values } => RowWriteOp::Update {
                row: *row,
                values: values.clone(),
            },
            WriteOp::Delete { row } => RowWriteOp::Delete { row: *row },
        })
        .collect()
}

/// One sentence for a batch the driver cannot apply atomically (step 33.1,
/// item M20): non-atomic drivers accept single-operation batches only.
fn atomic_batch_reason(db: &crate::state::DatabaseState, ops: usize) -> Option<String> {
    (!db.capabilities.atomic_batch && ops > 1).then(|| {
        format!(
            "unsupported: the {} driver applies journal batches one operation at a time — a multi-op batch would not be atomic (split it into single-operation batches)",
            db.driver
        )
    })
}

fn out_of_range(table: &TableDef, row: usize, count: usize) -> String {
    format!(
        "row {row} of {:?} is out of range ({count} rows)",
        table.name
    )
}

impl crate::core::Core {
    /// Previews a write batch and stores it as the pending journal.
    pub(crate) fn preview_write(&mut self, table: &str, ops: Vec<WriteOp>) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        // ADR-7: the dispatcher refuses before the port is asked (L14).
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        if let Some(reason) = atomic_batch_reason(db, ops.len()) {
            return failed(FailureCode::Unsupported, reason);
        }
        let Some(def) = db
            .tables
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
        else {
            return failed(
                FailureCode::NotFound,
                crate::port::PortError::TableNotFound(table.to_owned()).to_string(),
            );
        };
        let def = def.clone();
        let rows = match self.port.read_rows(
            &def.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        let (prepared, changes) = match preview(&def, &rows, &ops) {
            Ok(result) => result,
            Err(reason) => return failed(FailureCode::InvalidInput, reason),
        };
        let token = self.state.next_write_token;
        self.state.next_write_token += 1;
        self.state.pending_write = Some(PendingWrite {
            table: def.name.clone(),
            token,
            ops: prepared,
            changes: changes.clone(),
        });
        vec![Event::WritePreviewed {
            table: def.name,
            token,
            changes,
        }]
    }

    /// Applies the pending journal after re-validating its preview.
    pub(crate) fn apply_write(&mut self, token: u64) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(pending) = self.state.pending_write.clone() else {
            return failed(
                FailureCode::Lifecycle,
                "no pending write journal — preview a batch first",
            );
        };
        if pending.token != token {
            return failed(
                FailureCode::Stale,
                format!(
                    "stale write token {token} (pending journal is {})",
                    pending.token
                ),
            );
        }
        if let Some(db) = self.state.database.as_ref()
            && let Some(reason) = atomic_batch_reason(db, pending.ops.len())
        {
            return failed(FailureCode::Unsupported, reason);
        }
        let Some(def) = self
            .state
            .database
            .as_ref()
            .and_then(|db| {
                db.tables
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(&pending.table))
            })
            .cloned()
        else {
            return failed(
                FailureCode::NotFound,
                crate::port::PortError::TableNotFound(pending.table.clone()).to_string(),
            );
        };
        let rows = match self.port.read_rows(
            &def.name,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        ) {
            Ok(rows) => rows,
            Err(e) => return failed(FailureCode::Port, e.to_string()),
        };
        let fresh = match preview(&def, &rows, &pending.ops) {
            Ok((_, changes)) => changes,
            Err(reason) => {
                return failed(
                    FailureCode::Stale,
                    format!("the table changed since the preview: {reason}"),
                );
            }
        };
        if fresh != pending.changes {
            return failed(
                FailureCode::Stale,
                "the table changed since the preview — preview the batch again",
            );
        }
        let rollback = rollback_ops(&pending.changes);
        let ops = to_row_write_ops(&pending.ops);
        match self.port.write_row_ops(&pending.table, &ops) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                let count = |kind: ChangeKind| {
                    pending
                        .changes
                        .iter()
                        .filter(|change| change.kind == kind)
                        .count()
                };
                self.state.pending_write = None;
                self.state.last_applied = Some(AppliedWrite {
                    table: pending.table.clone(),
                    token,
                    rollback: rollback.clone(),
                });
                vec![Event::WriteApplied {
                    table: pending.table,
                    token,
                    inserted: count(ChangeKind::Insert),
                    updated: count(ChangeKind::Update),
                    deleted: count(ChangeKind::Delete),
                    rollback,
                }]
            }
        }
    }

    /// Rolls the last applied journal back (inverse operations).
    pub(crate) fn rollback_write(&mut self, token: u64) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        if !db.capabilities.write_rows {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::WriteRows),
                DriverCapabilities::unsupported(&db.driver, "write rows for this source"),
            );
        }
        let Some(applied) = self.state.last_applied.clone() else {
            return failed(
                FailureCode::Lifecycle,
                "no applied write journal to roll back",
            );
        };
        if applied.token != token {
            return failed(
                FailureCode::Stale,
                format!(
                    "stale rollback token {token} (last applied journal is {})",
                    applied.token
                ),
            );
        }
        let ops = to_row_write_ops(&applied.rollback);
        match self.port.write_row_ops(&applied.table, &ops) {
            Err(e) => failed(FailureCode::Port, e.to_string()),
            Ok(()) => {
                self.state.last_applied = None;
                vec![Event::WriteRolledBack {
                    table: applied.table,
                    token,
                }]
            }
        }
    }

    /// Discards the pending journal without writing anything.
    pub(crate) fn discard_write(&mut self, token: u64) -> Vec<Event> {
        let Some(pending) = self.state.pending_write.as_ref() else {
            return failed(
                FailureCode::Lifecycle,
                "no pending write journal to discard",
            );
        };
        if pending.token != token {
            return failed(
                FailureCode::Stale,
                format!(
                    "stale write token {token} (pending journal is {})",
                    pending.token
                ),
            );
        }
        self.state.pending_write = None;
        vec![Event::WriteDiscarded { token }]
    }
}
