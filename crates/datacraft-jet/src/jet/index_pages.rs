//! Jet4 index leaf pages (step 49.2, item M63).
//!
//! The engine's catalog is resolved through the `ParentIdName`/`Id` B-trees,
//! so a catalog row the writer appends is invisible until the corresponding
//! leaf carries its entry (evidence in `docs/FORMAT-JET.md`, "Fresh-create
//! template and the engine's open gates"). This module decodes, extends and
//! re-encodes single-page leaf roots byte-exactly; page splitting remains a
//! roadmap item (M15) and an overfull leaf is refused precisely.

#[cfg(test)]
mod tests;
