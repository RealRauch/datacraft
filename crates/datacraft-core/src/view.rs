//! Sidecar view designs (step 71.1, item M93).
//!
//! A view is a named `SELECT` stored in the sidecar (ADR-1 design
//! authority); execution follows the sink/source rule — SQLite applies it
//! natively, while Jet/ACE/CSV run it through the shadow SQL engine.

use serde::{Deserialize, Serialize};

/// A named SELECT stored as a sidecar design object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDef {
    /// View name (unique against tables and other views, case-insensitive).
    pub name: String,
    /// The SELECT statement (SQLite dialect, executed by the shadow engine
    /// on Jet/ACE/CSV and applied natively on SQLite).
    pub sql: String,
}

impl ViewDef {
    /// Creates a view definition.
    pub fn new(name: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
        }
    }

    /// Whether the statement is a `SELECT`/`WITH` query — the shape gate the
    /// create/update commands enforce.
    pub fn is_select(&self) -> bool {
        self.sql.split_whitespace().next().is_some_and(|word| {
            word.eq_ignore_ascii_case("select") || word.eq_ignore_ascii_case("with")
        })
    }
}

/// Sidecar view commands (step 71.1, item M93) — one variant per action,
/// carried as `Command::View`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
#[non_exhaustive]
pub enum ViewCommand {
    /// Creates a named SELECT view (unique against tables and views).
    Create {
        /// View name.
        name: String,
        /// The SELECT statement.
        sql: String,
    },
    /// Replaces a view's SELECT statement.
    SetSql {
        /// Target view name (case-insensitive).
        name: String,
        /// The new SELECT statement.
        sql: String,
    },
    /// Deletes a view and closes its tabs.
    Delete {
        /// Target view name (case-insensitive).
        name: String,
    },
}
