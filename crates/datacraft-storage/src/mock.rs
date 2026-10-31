//! In-memory reference backend (step 2.1, item H4).
//!
//! `InMemoryEngine` is the executable specification of the [`StorageEngine`]
//! contract: the boundary tests run against it, and later crates (headless
//! core, E2E harness) reuse it wherever no real file is needed. It is not a
//! stand-in for the Jet/ACE backends — it is the contract's reference
//! implementation.

use std::path::Path;

use crate::engine::StorageEngine;
use crate::error::StorageError;
use crate::types::{
    DatabaseMeta, FormatVersion, Row, RowBatch, RowOp, RowRange, TableSchema, WriteReport,
};

#[derive(Debug)]
struct StoredTable {
    schema: TableSchema,
    rows: Vec<Row>,
}

/// Reference in-memory backend.
#[derive(Debug)]
pub struct InMemoryEngine {
    format: FormatVersion,
    tables: Vec<StoredTable>,
}

impl InMemoryEngine {
    /// Creates an empty in-memory database of the given format version.
    pub fn new(format: FormatVersion) -> Self {
        Self {
            format,
            tables: Vec::new(),
        }
    }

    /// Mock-only helper: registers a table definition. Duplicate names are a
    /// programming error and panic.
    pub fn create_table(&mut self, schema: TableSchema) {
        assert!(
            !self.tables.iter().any(|t| t.schema.name == schema.name),
            "duplicate table name in mock: {}",
            schema.name
        );
        self.tables.push(StoredTable {
            schema,
            rows: Vec::new(),
        });
    }

    fn table(&self, name: &str) -> Result<&StoredTable, StorageError> {
        self.tables
            .iter()
            .find(|t| t.schema.name == name)
            .ok_or_else(|| StorageError::TableNotFound(name.to_owned()))
    }

    fn table_mut(&mut self, name: &str) -> Result<&mut StoredTable, StorageError> {
        self.tables
            .iter_mut()
            .find(|t| t.schema.name == name)
            .ok_or_else(|| StorageError::TableNotFound(name.to_owned()))
    }
}

impl StorageEngine for InMemoryEngine {
    fn open(&mut self, _path: &Path) -> Result<DatabaseMeta, StorageError> {
        Ok(DatabaseMeta::new(self.format))
    }

    fn schema(&self) -> Result<Vec<TableSchema>, StorageError> {
        Ok(self.tables.iter().map(|t| t.schema.clone()).collect())
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<RowBatch, StorageError> {
        let stored = self.table(table)?;
        if range.start > stored.rows.len() {
            return Err(StorageError::OutOfRange {
                table: table.to_owned(),
                start: range.start,
                len: range.len,
                row_count: stored.rows.len(),
            });
        }
        let end = (range.start + range.len).min(stored.rows.len());
        Ok(RowBatch {
            start_row: range.start,
            rows: stored.rows[range.start..end].to_vec(),
        })
    }

    fn write_rows(&mut self, table: &str, ops: &[RowOp]) -> Result<WriteReport, StorageError> {
        let arity = self.table(table)?.schema.column_count();
        let stored = self.table_mut(table)?;
        let mut report = WriteReport {
            inserted: 0,
            updated: 0,
            deleted: 0,
        };
        for op in ops {
            match op {
                RowOp::Insert(values) => {
                    Self::check_arity(table, values.len(), arity)?;
                    stored.rows.push(values.clone());
                    report.inserted += 1;
                }
                RowOp::Update { row, values } => {
                    Self::check_row_index(table, *row, stored.rows.len())?;
                    Self::check_arity(table, values.len(), arity)?;
                    stored.rows[*row] = values.clone();
                    report.updated += 1;
                }
                RowOp::Delete { row } => {
                    Self::check_row_index(table, *row, stored.rows.len())?;
                    stored.rows.remove(*row);
                    report.deleted += 1;
                }
            }
        }
        Ok(report)
    }

    fn flush(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

impl InMemoryEngine {
    fn check_row_index(table: &str, row: usize, row_count: usize) -> Result<(), StorageError> {
        if row >= row_count {
            return Err(StorageError::OutOfRange {
                table: table.to_owned(),
                start: row,
                len: 1,
                row_count,
            });
        }
        Ok(())
    }

    fn check_arity(table: &str, values: usize, arity: usize) -> Result<(), StorageError> {
        if values != arity {
            return Err(StorageError::SchemaViolation {
                table: table.to_owned(),
                detail: format!(
                    "operation carries {values} values but the table has {arity} columns"
                ),
            });
        }
        Ok(())
    }
}
