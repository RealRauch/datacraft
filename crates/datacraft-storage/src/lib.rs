//! DataCraft storage — `StorageEngine` trait boundary and the pure-Rust
//! Jet/ACE backends (clean-room; `libmdb` serves as test-time oracle and only
//! in CI — see `docs/ARCHITECTURE.md` §6 and `docs/TESTING.md`).

pub mod detecting;
pub mod engine;
pub mod error;
pub mod jet;
pub(crate) mod mapping;
pub mod mock;
pub mod port_adapter;
pub(crate) mod port_errors;
pub(crate) mod port_native;
pub(crate) mod port_objects;
pub(crate) mod port_schema;
pub mod port_write;
pub mod sqlite;
pub mod types;

pub use engine::StorageEngine;
pub use error::StorageError;
pub use mock::InMemoryEngine;
pub use types::{
    ColumnSchema, DatabaseMeta, FormatVersion, Row, RowBatch, RowOp, RowRange, StorageDataType,
    StorageValue, TableSchema, WriteReport,
};
