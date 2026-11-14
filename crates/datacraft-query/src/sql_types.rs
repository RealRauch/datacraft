//! Data types of the SQL window (step 13.3, item H37; split from the
//! engine in step 31.2 so the snapshot vocabulary stays C-free).
//!
//! These types are part of the state and the wire in every build; only the
//! engine that *produces* a [`SqlResult`] sits behind this crate's
//! `sql-engine` feature (`rusqlite` is optional, step 76.4).

use serde::{Deserialize, Serialize};

use datacraft_types::{FieldValue, TableDef};

/// Columns plus positional rows of one SQL result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlResult {
    /// Result column names in select order.
    pub columns: Vec<String>,
    /// Rows, positional to `columns`.
    pub rows: Vec<Vec<FieldValue>>,
}

/// One materialized source: the table definition and all of its rows.
pub type Source = (TableDef, Vec<Vec<FieldValue>>);
