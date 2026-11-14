//! DataCraft type boundary — the pure Access domain vocabulary.
//!
//! This crate carries everything the engine's value and schema layers share
//! without depending on the command bus: the value model
//! ([`FieldValue`], [`ComplexRecord`]), the logical field kinds
//! ([`FieldKind`] and its precision families), table and field definitions,
//! field properties, the schema error taxonomy, and the pure helpers the
//! expression service and the criteria parser share (family-safe
//! [`compare`], OLE date math, value rendering).
//!
//! Dependency direction: `datacraft-core → datacraft-query →
//! datacraft-expr → datacraft-types`. Nothing here knows about commands,
//! ports, state or files — only serde is a dependency.
//!
//! `datacraft_core::schema` re-exports every public path of this crate, so
//! clients keep addressing values as `datacraft_core::schema::FieldValue`
//! (phase 76, items L36/L37).

mod compare;
mod date;
mod def;
mod display;
mod error;
mod field;
mod properties;
mod value;

pub use compare::compare;
pub use date::{civil_from_days, days_from_civil, parse_date};
pub use def::{FieldDef, TableDef, validate_object_name};
pub use display::{display_ole_datetime, display_value, field_kind_display};
pub use error::SchemaError;
pub use field::{AutoKind, BinaryKind, FieldKind, MultiValueKind, NumericKind};
pub use properties::{FieldProperties, IndexMode};
pub use value::{ComplexField, ComplexRecord, FieldValue};
