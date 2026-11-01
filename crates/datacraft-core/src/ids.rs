//! Identifiers of the headless core (step 4.1, item H13).

use serde::{Deserialize, Serialize};

/// Identifier of one open document tab. Monotonic per `Core` instance,
/// assigned by the reducer — never invented by clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TabId {
    value: u64,
}

impl TabId {
    /// Creates a tab id (reducer-internal; public for tests and snapshots).
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Numeric value of the id (stable across serialization).
    pub const fn value(self) -> u64 {
        self.value
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tab {}", self.value)
    }
}
