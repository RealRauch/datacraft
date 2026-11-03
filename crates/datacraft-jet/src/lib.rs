//! DataCraft Jet/ACE backend (crate split, step 31.2 / M19; ADR-7): the
//! clean-room pure-Rust Jet 3/4 and ACE read path plus the Jet4/ACE writer,
//! behind the core-owned
//! [`StoragePort`](datacraft_core::port::StoragePort) seam. `libmdb`
//! (MDB Tools) is a test-time CLI oracle only — it is never linked or
//! shipped (docs/ARCHITECTURE.md §12).

pub mod error;
pub mod jet;
pub(crate) mod mapping;
pub mod port_adapter;
pub(crate) mod port_errors;
pub(crate) mod port_native;
pub(crate) mod port_objects;
pub(crate) mod port_schema;
pub mod port_write;
pub mod types;

pub use error::StorageError;
pub use types::{
    ColumnSchema, DatabaseMeta, FormatVersion, Row, RowBatch, RowOp, RowRange, StorageDataType,
    StorageValue, TableSchema, WriteReport,
};
