//! Domain value type (step 3.1, item H10).
//!
//! `FieldValue` is the runtime counterpart of storage's physical
//! `StorageValue`: the command bus (Phase 4) and the schema validator speak
//! this type, storage maps its parsed values onto it at the engine boundary.

use serde::{Deserialize, Serialize};

/// One sub-record of a complex value (an attachment row, a multi-value
/// element or a version-history entry, step 16.1 / M4).
///
/// The flat backing table's internal link column (`_<name>`) and its
/// AutoNumber identity column are not part of the record — they are the
/// decoder's grouping keys, not user-visible data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexRecord {
    /// Sub-record fields in flat-table order.
    pub fields: Vec<ComplexField>,
}

/// One named field of a complex sub-record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexField {
    /// Field name as stored in the flat table.
    pub name: String,
    /// Field value.
    pub value: FieldValue,
}

impl ComplexRecord {
    /// Looks up a field by exact name.
    pub fn field(&self, name: &str) -> Option<&FieldValue> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    }

    /// The scalar used for display and QBE EXISTS predicates: `Value` for
    /// multi-value rows, `FileName` for attachment rows, otherwise the first
    /// non-binary field. `None` when the record has no scalar-shaped field.
    pub fn scalar(&self) -> Option<&FieldValue> {
        for preferred in ["Value", "FileName"] {
            if let Some(value) = self.field(preferred) {
                return Some(value);
            }
        }
        self.fields
            .iter()
            .find(|field| !matches!(field.value, FieldValue::Binary(_)))
            .map(|field| &field.value)
    }
}

/// A domain value of any field family; `Null` is a distinct value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldValue {
    /// SQL NULL.
    Null,
    /// Yes/No value.
    Boolean(bool),
    /// Integral value (Byte/Integer/Long Integer, widened).
    Integer(i64),
    /// Floating-point value (Single/Double).
    Float(f64),
    /// Currency in scaled integer form (factor 10⁻⁴).
    Currency(i64),
    /// Date/time in OLE Automation form (days since 1899-12-30).
    DateTime(f64),
    /// Fixed-point decimal in scaled integer form (scale from the field).
    Decimal(i128),
    /// Text value.
    Text(String),
    /// Raw binary value.
    Binary(Vec<u8>),
    /// 16-byte replication identifier.
    Guid([u8; 16]),
    /// ACE complex cell: the ordered sub-records of the backing table
    /// (attachments, multi-value elements or version-history entries).
    Complex(Vec<ComplexRecord>),
}

impl FieldValue {
    /// Value family name used in kind-mismatch diagnostics (mirrors
    /// [`FieldKind::family`](crate::FieldKind::family)).
    pub fn family(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::Currency(_) => "currency",
            Self::DateTime(_) => "date/time",
            Self::Decimal(_) => "decimal",
            Self::Text(_) => "text",
            Self::Binary(_) => "binary",
            Self::Guid(_) => "guid",
            Self::Complex(_) => "complex",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant reports its family name — mismatch diagnostics name
    /// both sides, so all families must be stable.
    #[test]
    fn family_names_cover_every_variant() {
        let cases: [(FieldValue, &str); 11] = [
            (FieldValue::Null, "null"),
            (FieldValue::Boolean(true), "boolean"),
            (FieldValue::Integer(1), "integer"),
            (FieldValue::Float(1.0), "float"),
            (FieldValue::Currency(1), "currency"),
            (FieldValue::DateTime(1.0), "date/time"),
            (FieldValue::Decimal(1), "decimal"),
            (FieldValue::Text("t".into()), "text"),
            (FieldValue::Binary(vec![]), "binary"),
            (FieldValue::Guid([0; 16]), "guid"),
            (FieldValue::Complex(vec![]), "complex"),
        ];
        for (value, family) in cases {
            assert_eq!(value.family(), family, "family of {value:?}");
        }
    }
}
