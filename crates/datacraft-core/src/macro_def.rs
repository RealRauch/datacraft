//! Named macro inventory (step 17.1, item L4).
//!
//! Access stores named macros as binary definition blobs. The decoded
//! representation is deliberately **coarse**: the readable argument records
//! in storage order (expressions, SQL statements, object names). Action
//! names and execution semantics are outside the scope — DataCraft does not
//! execute macros (ADR-2, `docs/ROADMAP.md` §3).

use serde::{Deserialize, Serialize};

/// One named macro of the open database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroDef {
    /// Macro name from the `MSysObjects` catalog.
    pub name: String,
    /// Readable definition arguments in storage order (empty when the
    /// definition blob is not decoded).
    pub actions: Vec<MacroAction>,
    /// Whether the definition blob was decoded. `false` marks a name-only
    /// inventory entry (Jet3 `MSysAccessObjects` definitions are not
    /// decoded yet — BACKLOG remainder); it never hides the object.
    pub definition_decoded: bool,
}

/// One readable argument record of a macro definition.
///
/// The binary format interleaves numeric action records with the argument
/// records; only the readable arguments are exposed (the migration-relevant
/// content: embedded SQL, expressions, form/report names).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroAction {
    /// The readable argument text.
    pub argument: String,
}
