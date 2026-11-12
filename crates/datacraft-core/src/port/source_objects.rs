//! Read-only source inventory: indexes and triggers (phase 73, step 73.2;
//! item M90).
//!
//! These describe what the opened file itself carries. DataCraft lists
//! them for users and agents and never creates, drops or executes them
//! (trigger logic stays out of scope, ADR-2 analog).

use serde::{Deserialize, Serialize};

/// One index of the opened source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIndex {
    /// Table the index belongs to.
    pub table: String,
    /// Index name as the file reports it.
    pub name: String,
    /// Indexed columns in declaration order.
    pub columns: Vec<String>,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
    /// Whether the engine created it implicitly (SQLite's
    /// `sqlite_autoindex_*`: a PRIMARY KEY or UNIQUE constraint).
    pub automatic: bool,
}

/// One trigger of the opened source (inventory only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTrigger {
    /// Table the trigger is attached to.
    pub table: String,
    /// Trigger name as the file reports it.
    pub name: String,
    /// The defining statement verbatim.
    pub sql: String,
}
