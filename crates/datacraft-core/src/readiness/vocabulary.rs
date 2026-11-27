//! The readiness report's wire vocabulary (phase 103, step 103.1; item
//! M134): rings, object kinds, coverage verdicts, unsupported areas and
//! the assembled report document. Pure data — the classification lives
//! in the parent module, the VBA estimate in `estimate`.

use serde::{Deserialize, Serialize};

use crate::port::LinkedTable;

use super::estimate::VbaModuleEstimate;

/// The compatibility ring an object's role lives in (roadmap §2). Ring 1
/// (expressions) and Ring 5 (VBA) have no objects of their own —
/// expressions surface inside forms/reports/queries, VBA modules report
/// through the VBA section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ring {
    /// Ring 0 — storage: tables, rows and link records.
    Storage,
    /// Ring 2 — queries: sidecar QBE, views and the native-query subset.
    Query,
    /// Ring 3 — macros: read-only inventory (ADR-2).
    Macro,
    /// Ring 4 — the form/report runtime and print preview.
    Runtime,
}

/// The kind of an assessed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportObjectKind {
    /// A local, readable table.
    Table,
    /// A linked-table record (inventory only).
    LinkedTable,
    /// A sidecar QBE query.
    Query,
    /// A native saved query from the file.
    NativeQuery,
    /// A sidecar view design.
    View,
    /// A file-owned view.
    NativeView,
    /// A sidecar form design.
    Form,
    /// A catalog-named native form (undecoded).
    NativeForm,
    /// A sidecar report design.
    Report,
    /// A catalog-named native report (undecoded).
    NativeReport,
    /// A named macro (inventory only).
    Macro,
}

/// Coverage verdict for one object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum ObjectStatus {
    /// The object's role is covered by the shipped rings.
    Runs,
    /// Outside the shipped rings; the reason names the gap and, where
    /// one exists, the migration path.
    NotCovered {
        /// The precise, locatable gap statement (English).
        reason: String,
    },
}

/// One assessed object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectReadiness {
    /// Object name as the database reports it.
    pub name: String,
    /// Object kind.
    pub kind: ReportObjectKind,
    /// The ring the object's role lives in.
    pub ring: Ring,
    /// The coverage verdict.
    pub status: ObjectStatus,
}

/// Stable area key of a cross-cutting unsupported feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedArea {
    /// Macros exist; inventoried, never executed (ADR-2).
    MacroExecution,
    /// A VBA project exists; extracted and analyzed, never executed.
    VbaExecution,
    /// Linked tables exist; targets stay unresolved (roadmap §9).
    LinkedTables,
    /// Native form/report definitions exist and stay undecoded (M89).
    NativeDefinitions,
    /// An inventory decode failed at open; the report under-counts.
    DecodeError,
}

/// One cross-cutting unsupported feature. Per-object gaps live in the
/// object statuses; this list aggregates the boundaries that affect the
/// whole application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    /// The stable area key.
    pub area: UnsupportedArea,
    /// The precise boundary statement (English).
    pub reason: String,
}

/// The VBA section of the report (roadmap §3: extraction and static
/// analysis only — execution stays out of scope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaReadiness {
    /// Per module, in project order; empty without a VBA project.
    pub modules: Vec<VbaModuleEstimate>,
    /// Code lines across all modules.
    pub code_lines: usize,
    /// Candidate share over code lines, percent floored; `None` when no
    /// module carries code.
    pub transplicable_percent: Option<u8>,
}

/// The migration-effort summary behind the per-object evidence.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EffortSummary {
    /// Assessed objects.
    pub objects_total: usize,
    /// Objects whose role runs with the shipped rings.
    pub running: usize,
    /// Objects outside the shipped rings.
    pub not_covered: usize,
    /// Native form/report catalog entries (Route B migration targets).
    pub native_designs: usize,
    /// Native saved queries.
    pub native_queries: usize,
    /// Native queries outside the executable subset (or unresolved).
    pub native_queries_not_executable: usize,
    /// Named macros.
    pub macros: usize,
    /// Linked-table records.
    pub linked_tables: usize,
    /// VBA modules.
    pub vba_modules: usize,
}

/// The full readiness report of one open database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppReadiness {
    /// Every object of the open database, in the state's canonical kind
    /// order (tables, links, queries, views, forms, reports, macros).
    pub objects: Vec<ObjectReadiness>,
    /// The VBA estimate.
    pub vba: VbaReadiness,
    /// The linked-table inventory (phase 100), verbatim.
    pub links: Vec<LinkedTable>,
    /// Cross-cutting unsupported features.
    pub unsupported: Vec<UnsupportedFeature>,
    /// The effort summary.
    pub effort: EffortSummary,
}
