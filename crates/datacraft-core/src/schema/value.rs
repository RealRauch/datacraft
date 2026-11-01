//! Domain value type (step 3.1, item H10).
//!
//! `FieldValue` is the runtime counterpart of storage's physical
//! `StorageValue`: the command bus (Phase 4) and the schema validator speak
//! this type, storage maps its parsed values onto it at the engine boundary.

use serde::{Deserialize, Serialize};

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
}

impl FieldValue {
    /// Value family name used in kind-mismatch diagnostics (mirrors
    /// [`FieldKind::family`](super::field::FieldKind::family)).
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
        }
    }
}
