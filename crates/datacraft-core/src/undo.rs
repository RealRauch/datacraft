//! Row-write undo/redo history (step 28.1, item U14).
//!
//! The stack is per open database and bounded; it records **row writes
//! only** (`InsertRow`/`UpdateRow`/`DeleteRow`). Schema changes, journal
//! batches and file-level commands are barriers: they clear the history,
//! because their row effects cannot be expressed as row images. Undo and
//! redo are ordinary bus commands, so GUI, CLI and MCP share them.
//!
//! Operations address rows by their full value image, never by an index:
//! an undo that appends a deleted row shifts the indices behind it, so
//! image matching is the only deterministic way to keep redo correct.

use serde::{Deserialize, Serialize};

use crate::core::failed;
use crate::event::Event;
use crate::port::{DriverCapabilities, RowRange};
use crate::schema::FieldValue;

/// Maximum number of recorded row writes per open database.
pub const MAX_HISTORY: usize = 50;

/// One row operation of the history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HistoryOp {
    /// Appends a row image verbatim, including engine-assigned AutoNumber
    /// values ([`crate::port::StoragePort::restore_row`]).
    Restore {
        /// The exact row image.
        values: Vec<FieldValue>,
    },
    /// Deletes the first row matching the image.
    Delete {
        /// The exact row image.
        values: Vec<FieldValue>,
    },
    /// Replaces the first row matching `from` with `to`.
    Update {
        /// The image the row must currently carry.
        from: Vec<FieldValue>,
        /// The image to write.
        to: Vec<FieldValue>,
    },
}

/// One undoable row write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Canonical table name.
    pub table: String,
    /// Operations in undo order.
    pub undo: Vec<HistoryOp>,
    /// Operations in redo order.
    pub redo: Vec<HistoryOp>,
}

impl crate::core::Core {
    /// Undoes the last recorded row write (step 28.1, item U14).
    pub(crate) fn undo(&mut self) -> Vec<Event> {
        self.move_history(true)
    }

    /// Re-applies the last undone row write (step 28.1, item U14).
    pub(crate) fn redo(&mut self) -> Vec<Event> {
        self.move_history(false)
    }

    fn move_history(&mut self, undo: bool) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        if !db.capabilities.write_rows {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "write rows for this source",
            ));
        }
        let entry = {
            let db = self.state.database.as_mut().expect("checked above");
            if undo { db.undo.pop() } else { db.redo.pop() }
        };
        let Some(entry) = entry else {
            return failed(if undo {
                "nothing to undo"
            } else {
                "nothing to redo"
            });
        };
        let ops = if undo {
            entry.undo.clone()
        } else {
            entry.redo.clone()
        };
        match self.apply_history(&entry.table, &ops) {
            Ok(()) => {
                if let Some(db) = self.state.database.as_mut() {
                    if undo {
                        db.redo.push(entry.clone());
                    } else {
                        db.undo.push(entry.clone());
                        if db.undo.len() > MAX_HISTORY {
                            db.undo.remove(0);
                        }
                    }
                }
                vec![if undo {
                    Event::Undone { table: entry.table }
                } else {
                    Event::Redone { table: entry.table }
                }]
            }
            Err(reason) => {
                // The stack did not change: a stale entry is retried once
                // and then reported again instead of silently vanishing.
                if let Some(db) = self.state.database.as_mut() {
                    if undo {
                        db.undo.push(entry);
                    } else {
                        db.redo.push(entry);
                    }
                }
                failed(reason)
            }
        }
    }

    fn apply_history(&mut self, table: &str, ops: &[HistoryOp]) -> Result<(), String> {
        let known = self
            .state
            .database
            .as_ref()
            .is_some_and(|db| db.tables.iter().any(|t| t.name.eq_ignore_ascii_case(table)));
        if !known {
            return Err(crate::port::PortError::TableNotFound(table.to_owned()).to_string());
        }
        for op in ops {
            match op {
                HistoryOp::Restore { values } => {
                    self.port
                        .restore_row(table, values)
                        .map_err(|e| e.to_string())?;
                }
                HistoryOp::Delete { values } => {
                    let index = self.find_row(table, values)?;
                    self.port
                        .delete_row(table, index)
                        .map_err(|e| e.to_string())?;
                }
                HistoryOp::Update { from, to } => {
                    let index = self.find_row(table, from)?;
                    self.port
                        .update_row(table, index, to)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    fn find_row(&self, table: &str, image: &[FieldValue]) -> Result<usize, String> {
        let rows = self
            .port
            .read_rows(
                table,
                RowRange {
                    start: 0,
                    len: usize::MAX,
                },
            )
            .map_err(|e| e.to_string())?;
        rows.iter()
            .position(|row| row == image)
            .ok_or_else(|| format!("undo history is stale — no row matches {image:?} in {table:?}"))
    }

    /// Records a successful insert: the appended row is the last row of
    /// the table (the port's insert contract), so its actual image — with
    /// the engine-assigned AutoNumber — is read back.
    pub(crate) fn record_insert(&mut self, table: &str) {
        let rows = self.port.read_rows(
            table,
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        );
        let Ok(rows) = rows else {
            self.clear_history();
            return;
        };
        let Some(image) = rows.last().cloned() else {
            self.clear_history();
            return;
        };
        self.push_history(HistoryEntry {
            table: table.to_owned(),
            undo: vec![HistoryOp::Delete {
                values: image.clone(),
            }],
            redo: vec![HistoryOp::Restore { values: image }],
        });
    }

    /// Records a successful update (before/after images).
    pub(crate) fn record_update(
        &mut self,
        table: &str,
        before: &[FieldValue],
        after: &[FieldValue],
    ) {
        self.push_history(HistoryEntry {
            table: table.to_owned(),
            undo: vec![HistoryOp::Update {
                from: after.to_vec(),
                to: before.to_vec(),
            }],
            redo: vec![HistoryOp::Update {
                from: before.to_vec(),
                to: after.to_vec(),
            }],
        });
    }

    /// Records a successful delete (undo restores the before image).
    pub(crate) fn record_delete(&mut self, table: &str, before: &[FieldValue]) {
        self.push_history(HistoryEntry {
            table: table.to_owned(),
            undo: vec![HistoryOp::Restore {
                values: before.to_vec(),
            }],
            redo: vec![HistoryOp::Delete {
                values: before.to_vec(),
            }],
        });
    }

    fn push_history(&mut self, entry: HistoryEntry) {
        if let Some(db) = self.state.database.as_mut() {
            db.redo.clear();
            db.undo.push(entry);
            if db.undo.len() > MAX_HISTORY {
                db.undo.remove(0);
            }
        }
    }

    /// Clears both stacks — the barrier for schema, batch and file-level
    /// writes whose row effects are not row images.
    pub(crate) fn clear_history(&mut self) {
        if let Some(db) = self.state.database.as_mut() {
            db.undo.clear();
            db.redo.clear();
        }
    }
}
