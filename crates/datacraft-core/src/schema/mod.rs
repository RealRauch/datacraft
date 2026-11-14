//! Access domain schema model (Phase 3, ARCHITECTURE §7).
//!
//! Logical Access data types, typed field properties and their validation
//! semantics. This is the vocabulary the command bus (Phase 4), the UI and
//! the sidecar document (step 3.2, ADR-1) share; the storage backends map
//! their physical types onto it.
//!
//! The vocabulary itself lives in [`datacraft_types`] (phase 76, item L36);
//! this module re-exports every public path and keeps the sidecar document,
//! which needs the command-layer object models.

mod document;

pub(crate) use datacraft_types::validate_object_name;
pub use datacraft_types::{
    AutoKind, BinaryKind, ComplexField, ComplexRecord, FieldDef, FieldKind, FieldProperties,
    FieldValue, IndexMode, MultiValueKind, NumericKind, SchemaError, TableDef,
};
pub use document::{SCHEMA_VERSION, SchemaDocument};
