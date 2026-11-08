//! The command payload helpers (split from `command.rs` for the size
//! gate; re-exported under `datacraft_core::command`, so paths and the
//! wire shape are unchanged).

use serde::{Deserialize, Serialize};

/// Row context of `Command::RunExpression` (step 15.2, H41).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionContext {
    /// Source table name (case-insensitive).
    pub table: String,
    /// Zero-based index in the table's live-row order.
    pub row: usize,
}
