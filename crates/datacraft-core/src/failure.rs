//! Stable machine-readable failure codes (step 62.1, item I6).
//!
//! Every `Event::Failed` carries a code alongside its English reason
//! text: agents key on the code (data, not prose — the same principle as
//! I6), the UI localizes known codes and falls back to the English text
//! unchanged for everything else (ADR-4: language stays UI-owned; core
//! carries no localized text). The vocabulary is coarse by design — one
//! variant per failure CLASS, never per message — so it stays stable
//! while reasons keep their precision.

use serde::{Deserialize, Serialize};

use crate::availability::DisabledReason;

/// The failure class of one `Event::Failed` (step 62.1, item I6).
/// `#[non_exhaustive]` (M22): a newer core may add classes; consumers
/// match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FailureCode {
    /// No database is open (or the command needs one first).
    NoDatabase,
    /// A named object (table, form, report, query, field, module) does
    /// not exist in the open database.
    NotFound,
    /// A name collision: the object or field already exists.
    Duplicate,
    /// The command input is invalid or incomplete (empty names, missing
    /// prerequisites, malformed payloads).
    InvalidInput,
    /// An index or page reference lies outside the live domain.
    OutOfRange,
    /// The addressed state changed since it was previewed (preview
    /// tokens, stale sidecars).
    Stale,
    /// The operation conflicts with the current lifecycle (a database is
    /// already open, no journal is pending, no tabs are open).
    Lifecycle,
    /// An expression or source payload does not parse.
    Parse,
    /// The path is not supported for this source or build (no capability
    /// vocabulary applies here — see [`FailureCode::Capability`]).
    Unsupported,
    /// A capability gate refused the command; the reason mirrors the
    /// affordance vocabulary (ADR-7, M17).
    Capability(DisabledReason),
    /// The storage port answered a failure; the reason carries its
    /// precise text.
    Port,
    /// The design sidecar (`.datacraft`) failed to load or validate.
    DesignDocument,
    /// A filesystem operation failed.
    Io,
    /// A failure class this build does not name yet (wildcard home —
    /// exists so every failure has a stable code from day one).
    Other,
}

impl FailureCode {
    /// The stable wire spelling (the serde form; handy for logs and for
    /// tests that assert on data).
    pub fn as_str(self) -> &'static str {
        match self {
            FailureCode::NoDatabase => "no_database",
            FailureCode::NotFound => "not_found",
            FailureCode::Duplicate => "duplicate",
            FailureCode::InvalidInput => "invalid_input",
            FailureCode::OutOfRange => "out_of_range",
            FailureCode::Stale => "stale",
            FailureCode::Lifecycle => "lifecycle",
            FailureCode::Parse => "parse",
            FailureCode::Unsupported => "unsupported",
            FailureCode::Capability(_) => "capability",
            FailureCode::Port => "port",
            FailureCode::DesignDocument => "design_document",
            FailureCode::Io => "io",
            FailureCode::Other => "other",
        }
    }
}
