//! Access domain schema model (Phase 3, ARCHITECTURE §7).
//!
//! Logical Access data types, typed field properties and their validation
//! semantics. This is the vocabulary the command bus (Phase 4), the UI and
//! the sidecar document (step 3.2, ADR-1) share; the storage backends map
//! their physical types onto it.

mod def;
mod document;
mod error;
mod field;
mod properties;
mod value;

pub(crate) use def::validate_object_name;
pub use def::{FieldDef, TableDef};
pub use document::{SCHEMA_VERSION, SchemaDocument};
pub use error::SchemaError;
pub use field::{AutoKind, BinaryKind, FieldKind, MultiValueKind, NumericKind};
pub use properties::{FieldProperties, IndexMode};
pub use value::{ComplexField, ComplexRecord, FieldValue};
