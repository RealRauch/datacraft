//! Cell-level edit payloads (step 86.3, item M101): one cell of one row,
//! addressed by index, with the domain value to write.

use crate::schema::FieldValue;

/// One cell edit of [`Command::UpdateCells`](crate::command::Command):
/// the zero-based live-row index, the zero-based column index and the
/// value to write (`Null` clears the cell where the driver allows it).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CellEdit {
    /// Zero-based index in the table's live-row order.
    pub row: usize,
    /// Zero-based column index.
    pub col: usize,
    /// The value to write.
    pub value: FieldValue,
}
