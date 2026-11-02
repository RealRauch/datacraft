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
        failed("undo history is not implemented")
    }

    /// Re-applies the last undone row write (step 28.1, item U14).
    pub(crate) fn redo(&mut self) -> Vec<Event> {
        failed("redo history is not implemented")
    }
}
