//! Physical -> domain value mapping (step 16.1, item M4).
//!
//! Shared by the port adapter and the complex-value decoder so both layers
//! speak one vocabulary (`StorageValue` below the boundary, `FieldValue`
//! above it). The variants correspond 1:1 — see docs/FORMAT-JET.md
//! "Domain mapping".

use datacraft_core::schema::FieldValue;

use crate::types::StorageValue;

/// Maps one stored value onto the domain vocabulary.
pub(crate) fn to_field_value(value: &StorageValue) -> FieldValue {
    match value {
        StorageValue::Null => FieldValue::Null,
        StorageValue::Bit(b) => FieldValue::Boolean(*b),
        StorageValue::Int(i) => FieldValue::Integer(*i),
        StorageValue::Float(f) => FieldValue::Float(*f),
        StorageValue::Currency(c) => FieldValue::Currency(*c),
        StorageValue::DateTime(d) => FieldValue::DateTime(*d),
        StorageValue::Text(s) => FieldValue::Text(s.clone()),
        StorageValue::Binary(b) => FieldValue::Binary(b.clone()),
        StorageValue::Guid(g) => FieldValue::Guid(*g),
        StorageValue::Decimal(d) => FieldValue::Decimal(*d),
    }
}
