//! Driver identity and declarative capabilities (phase 30, item M17;
//! ADR-7).
//!
//! Every data source names itself and reports what it can do *per opened
//! session* — capabilities depend on runtime state (a read-only file
//! disables the write capabilities). A capability reported `No` is a
//! contract: the matching port call answers a precise `Unsupported`, and
//! the dispatcher refuses the command before the port is called.

use serde::{Deserialize, Serialize};

/// Stable identity of the driver behind a [`StoragePort`](super::StoragePort)
/// (`"jet"`, `"sqlite"`, `"csv"`, …). Serializes transparently so the
/// snapshot spells it as a plain string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DriverId(String);

impl DriverId {
    /// Creates a driver id from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DriverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Support level of an optional capability. `Yes` means DataCraft's
/// commands can read *and* write the surface; `ReadOnly` means the
/// driver exposes it but no write path exists (or none yet); `No` means
/// the driver answers a precise `Unsupported`.
///
/// `#[non_exhaustive]` (M22, ADR-7): adding a level is not a breaking
/// change for drivers compiled against a published core — out-of-crate
/// consumers match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Support {
    /// The driver does not provide the surface at all.
    No,
    /// The driver provides it read-only.
    ReadOnly,
    /// The driver provides it for reads and writes.
    Yes,
}

/// Declarative capabilities of the open data source (ADR-7). Reported by
/// [`StoragePort::capabilities`](super::StoragePort::capabilities) and
/// carried verbatim in the state and the snapshot, so every client —
/// GUI, CLI, MCP agent — sees the same truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverCapabilities {
    /// Catalog and row reads are served (`tables`, `read_rows`).
    pub read_tables: bool,
    /// Row writes are served (`insert_row`, `update_row`, `delete_row`).
    pub write_rows: bool,
    /// Tables can be created (`create_table`).
    pub create_table: bool,
    /// Tables can be dropped (`drop_table`).
    pub drop_table: bool,
    /// Database files can be created (`create_database`).
    pub create_database: bool,
    /// Schemas can be altered in place (`alter_table`).
    pub alter_table: bool,
    /// A batch via `write_row_ops` applies atomically — either every
    /// operation lands or none does.
    pub atomic_batch: bool,
    /// The source honours database passwords on open.
    pub passwords: bool,
    /// Engine-assigned AutoNumber columns are modelled.
    pub autonumber: Support,
    /// Referential relationships are exposed.
    pub relationships: Support,
    /// Native saved queries are exposed.
    pub native_queries: Support,
    /// Named macros are exposed as a read-only inventory.
    pub macros: Support,
    /// An extracted VBA project is exposed (never executed, ADR-2).
    pub vba: Support,
    /// Reserved vocabulary (ADR-7): no driver reports `Yes` yet — no
    /// command exists for triggers.
    pub triggers: Support,
    /// Reserved vocabulary (ADR-7): no driver reports `Yes` yet — no
    /// command exists for stored procedures.
    pub stored_procedures: Support,
}

impl DriverCapabilities {
    /// The conservative declaration (M22): every boolean capability is
    /// absent and every optional surface is `No`. This is the honest
    /// starting point for a minimal driver — start here and state the
    /// deviations, because an undeclared surface answers a precise
    /// `Unsupported` instead of a broken call.
    pub fn none() -> Self {
        Self {
            read_tables: false,
            write_rows: false,
            create_table: false,
            drop_table: false,
            create_database: false,
            alter_table: false,
            atomic_batch: false,
            passwords: false,
            autonumber: Support::No,
            relationships: Support::No,
            native_queries: Support::No,
            macros: Support::No,
            vba: Support::No,
            triggers: Support::No,
            stored_procedures: Support::No,
        }
    }

    /// The optimistic reference set: every surface DataCraft's commands
    /// can drive is reported as available, except batch atomicity (the
    /// trait's default batch loop applies operations one by one) and the
    /// reserved vocabulary. Not a trait default (M22) — a driver must
    /// state its own declaration; this constructor serves tests, the
    /// mock port and drivers that genuinely cover the full surface.
    pub fn full() -> Self {
        Self {
            read_tables: true,
            write_rows: true,
            create_table: true,
            drop_table: true,
            create_database: true,
            alter_table: true,
            atomic_batch: false,
            passwords: true,
            autonumber: Support::No,
            relationships: Support::ReadOnly,
            native_queries: Support::ReadOnly,
            macros: Support::ReadOnly,
            vba: Support::ReadOnly,
            triggers: Support::No,
            stored_procedures: Support::No,
        }
    }

    /// The stable failure reason for a capability the driver reported as
    /// absent — the same `Failed` shape a port refusal produces.
    pub fn unsupported(driver: &DriverId, what: &str) -> String {
        format!("unsupported: the {driver} driver cannot {what}")
    }
}
