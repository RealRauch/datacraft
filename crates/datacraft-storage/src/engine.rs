//! The `StorageEngine` trait — the single boundary between DataCraft and any
//! database file backend (step 2.1, item H4; ARCHITECTURE §6).

use std::path::Path;

use crate::error::StorageError;
use crate::types::{DatabaseMeta, RowBatch, RowOp, RowRange, TableSchema, WriteReport};

/// Backend-agnostic storage boundary.
///
/// Contract rules (enforced by the boundary tests in `tests/engine_boundary.rs`):
///
/// - `schema` returns the user-visible tables in deterministic (creation) order.
/// - `read_rows` returns at most `range.len` rows starting at `range.start`;
///   a window starting at `row_count` yields an empty batch, anything beyond
///   fails with [`StorageError::OutOfRange`].
/// - `RowOp` indices address the row set as it evolves through the batch
///   (sequential application).
/// - Backends that cannot write return [`StorageError::Unsupported`].
/// - `flush` is idempotent.
pub trait StorageEngine {
    /// Opens the database at `path`; repeated calls re-attach.
    fn open(&mut self, path: &Path) -> Result<DatabaseMeta, StorageError>;

    /// Schema of all user-visible tables, deterministic order.
    fn schema(&self) -> Result<Vec<TableSchema>, StorageError>;

    /// Reads a window of rows from `table`.
    fn read_rows(&self, table: &str, range: RowRange) -> Result<RowBatch, StorageError>;

    /// Applies a sequential batch of row operations.
    fn write_rows(&mut self, table: &str, ops: &[RowOp]) -> Result<WriteReport, StorageError>;

    /// Persists all pending changes (idempotent).
    fn flush(&mut self) -> Result<(), StorageError>;
}
