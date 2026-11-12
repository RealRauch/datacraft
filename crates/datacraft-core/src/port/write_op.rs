//! Batch write vocabulary split from `port.rs` (L30, step 71.3).
//!
//! The journal's row operation enum and the shared window slice helper;
//! both stay `port::` items through the re-export.

use serde::{Deserialize, Serialize};

use crate::schema::FieldValue;

use super::{PortError, RowRange};

/// One row operation of a batch write (the journal's port vocabulary).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RowWriteOp {
    /// Appends a row.
    Insert(Vec<FieldValue>),
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

/// Window slice shared by implementations: start beyond the count is an
/// error, the tail window is empty.
pub fn window_slice(
    table: &str,
    range: RowRange,
    count: usize,
) -> Result<(usize, usize), PortError> {
    if range.start > count {
        return Err(PortError::OutOfRange {
            table: table.to_owned(),
            start: range.start,
            len: range.len,
            row_count: count,
        });
    }
    let end = range.start.saturating_add(range.len).min(count);
    Ok((range.start, end))
}
