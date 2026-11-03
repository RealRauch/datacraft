//! DataCraft storage — the pure-Rust Jet/ACE and SQLite backends behind the
//! core-owned [`StoragePort`](datacraft_core::port::StoragePort) seam (ADR-7;
//! clean-room; `libmdb` serves as test-time oracle and only in CI — see
//! `docs/ARCHITECTURE.md` §6 and `docs/TESTING.md`).

pub mod detecting;
pub mod error;
pub mod jet;
pub(crate) mod mapping;
pub mod port_adapter;
pub(crate) mod port_errors;
pub(crate) mod port_native;
pub(crate) mod port_objects;
pub(crate) mod port_schema;
pub mod port_write;
pub mod samples;
pub mod sqlite;
pub mod types;

pub use error::StorageError;
pub use types::{
    ColumnSchema, DatabaseMeta, FormatVersion, Row, RowBatch, RowOp, RowRange, StorageDataType,
    StorageValue, TableSchema, WriteReport,
};
