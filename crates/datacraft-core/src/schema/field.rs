//! Logical Access field kinds (step 3.1, item H10; ARCHITECTURE §7).
//!
//! `FieldKind` is the logical counterpart of the physical
//! [`StorageDataType`](../../datacraft_storage/enum.StorageDataType.html)
//! in storage: physical concerns live below the engine boundary, Access
//! semantics ("Short Text", "AutoNumber") live here. The Jet/ACE backends
//! map their physical types onto this model (dependency direction
//! `storage → core`).

use serde::{Deserialize, Serialize};

use super::error::SchemaError;
use super::value::FieldValue;

/// Smallest/largest OLE Automation day Access accepts (years 100–9999).
const OLE_DATE_MIN: f64 = -657_434.0;
/// Exclusive upper bound: `2958466.0` would be 10000-01-01.
const OLE_DATE_MAX_EXCLUSIVE: f64 = 2_958_466.0;

impl NumericKind {
    /// Value family name used in kind-mismatch diagnostics.
    fn family_name(self) -> &'static str {
        match self {
            Self::Byte | Self::Integer | Self::LongInteger => "integer",
            Self::Single => "single",
            Self::Double => "double",
            Self::Decimal { .. } => "decimal",
        }
    }

    /// Human-readable kind name used in range diagnostics.
    fn display_name(self) -> &'static str {
        match self {
            Self::Byte => "byte",
            Self::Integer => "integer",
            Self::LongInteger => "long integer",
            Self::Single => "single",
            Self::Double => "double",
            Self::Decimal { .. } => "decimal",
        }
    }

    /// Validates a domain value against this numeric precision family.
    pub fn validate_value(self, value: &FieldValue) -> Result<(), SchemaError> {
        let mismatch = || SchemaError::KindMismatch {
            expected: self.family_name(),
            found: value.family(),
        };
        match self {
            Self::Byte | Self::Integer | Self::LongInteger => match value {
                FieldValue::Integer(i) => {
                    let (min, max) = self.integer_range().expect("integral kinds have a range");
                    if *i >= min && *i <= max {
                        Ok(())
                    } else {
                        Err(SchemaError::ValueOutOfRange {
                            detail: format!(
                                "{} integers hold {min}..={max}, found {i}",
                                self.display_name()
                            ),
                        })
                    }
                }
                _ => Err(mismatch()),
            },
            Self::Single | Self::Double => match value {
                FieldValue::Float(f) if f.is_finite() => Ok(()),
                FieldValue::Float(_) => Err(SchemaError::ValueOutOfRange {
                    detail: "floating-point values must be finite".to_owned(),
                }),
                _ => Err(mismatch()),
            },
            Self::Decimal { .. } => match value {
                FieldValue::Decimal(_) => Ok(()),
                _ => Err(mismatch()),
            },
        }
    }
}

/// Precision family of a `Number` field, following the Access UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericKind {
    /// 1-byte unsigned integer (Access "Byte").
    Byte,
    /// 2-byte signed integer (Access "Integer").
    Integer,
    /// 4-byte signed integer (Access "Long Integer").
    LongInteger,
    /// IEEE 754 single precision (Access "Single").
    Single,
    /// IEEE 754 double precision (Access "Double").
    Double,
    /// Fixed-point decimal with declared precision and scale.
    Decimal {
        /// Total number of significant digits (1–28).
        precision: u8,
        /// Digits after the decimal point (0–precision).
        scale: u8,
    },
}

impl NumericKind {
    /// Inclusive integer range for the integral kinds; `None` otherwise.
    pub fn integer_range(self) -> Option<(i64, i64)> {
        match self {
            Self::Byte => Some((0, 255)),
            Self::Integer => Some((i64::from(i16::MIN), i64::from(i16::MAX))),
            Self::LongInteger => Some((i64::from(i32::MIN), i64::from(i32::MAX))),
            Self::Single | Self::Double | Self::Decimal { .. } => None,
        }
    }

    /// Validates the declared precision/scale of the decimal kind.
    pub fn validate(self) -> Result<(), SchemaError> {
        if let Self::Decimal { precision, scale } = self {
            if !(1..=28).contains(&precision) {
                return Err(SchemaError::InvalidKind {
                    detail: format!("decimal precision must be 1..=28, found {precision}"),
                });
            }
            if scale > precision {
                return Err(SchemaError::InvalidKind {
                    detail: format!(
                        "decimal scale must not exceed precision (precision {precision}, scale {scale})"
                    ),
                });
            }
        }
        Ok(())
    }
}

/// AutoNumber flavor (Access "Field Size" of an AutoNumber field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoKind {
    /// Incrementing 4-byte integer.
    LongInteger,
    /// 16-byte replication identifier (GUID).
    ReplicationId,
}

/// Binary field flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryKind {
    /// OLE object (embedded package or linked data).
    OleObject,
    /// ACE attachment (multi-value complex column).
    Attachment,
}

/// Logical Access type of a field (ARCHITECTURE §7 mapping table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum FieldKind {
    /// Short text with a maximum length of 1–255 characters.
    Text {
        /// Maximum number of characters (Access "Field Size").
        max_len: u16,
    },
    /// Long text stored on memo pages.
    Memo,
    /// Numeric field of the given precision family.
    Number(NumericKind),
    /// Date/time in OLE Automation form.
    DateTime,
    /// Currency as a scaled 64-bit integer (factor 10⁻⁴).
    Currency,
    /// AutoNumber — values are engine-assigned, never user-provided.
    AutoNumber(AutoKind),
    /// Yes/No. `tri_state` only governs UI display (Null display);
    /// actual nullability comes from `FieldProperties::required`.
    Boolean {
        /// Whether the UI offers a third (Null) display state.
        tri_state: bool,
    },
    /// Binary data (OLE object or attachment).
    Binary(BinaryKind),
}

impl FieldKind {
    /// Value family name used in kind-mismatch diagnostics.
    pub fn family(&self) -> &'static str {
        match self {
            Self::Text { .. } => "short text",
            Self::Memo => "memo",
            Self::Number(NumericKind::Byte | NumericKind::Integer | NumericKind::LongInteger) => {
                "integer"
            }
            Self::Number(NumericKind::Single) => "single",
            Self::Number(NumericKind::Double) => "double",
            Self::Number(NumericKind::Decimal { .. }) => "decimal",
            Self::DateTime => "date/time",
            Self::Currency => "currency",
            Self::AutoNumber(_) => "autonumber",
            Self::Boolean { .. } => "boolean",
            Self::Binary(_) => "binary",
        }
    }

    /// Validates the kind's own invariants (text length bounds, decimal
    /// precision/scale).
    pub fn validate(&self) -> Result<(), SchemaError> {
        match self {
            Self::Text { max_len } => {
                if !(1..=255).contains(max_len) {
                    return Err(SchemaError::InvalidKind {
                        detail: format!("text max_len must be 1..=255, found {max_len}"),
                    });
                }
                Ok(())
            }
            Self::Number(kind) => kind.validate(),
            _ => Ok(()),
        }
    }

    /// Validates a domain value against this kind. `Null` passes here —
    /// required-ness is a property concern checked on
    /// [`FieldDef::validate_value`](super::def::FieldDef::validate_value).
    pub fn validate_value(&self, value: &FieldValue) -> Result<(), SchemaError> {
        if matches!(value, FieldValue::Null) {
            return Ok(());
        }
        let mismatch = || SchemaError::KindMismatch {
            expected: self.family(),
            found: value.family(),
        };
        match self {
            Self::Text { max_len } => match value {
                FieldValue::Text(s) => {
                    let found = s.chars().count();
                    if found <= usize::from(*max_len) {
                        Ok(())
                    } else {
                        Err(SchemaError::ValueTooLong {
                            max_len: usize::from(*max_len),
                            found,
                        })
                    }
                }
                _ => Err(mismatch()),
            },
            Self::Memo => match value {
                FieldValue::Text(_) => Ok(()),
                _ => Err(mismatch()),
            },
            Self::Number(kind) => kind.validate_value(value),
            Self::DateTime => match value {
                FieldValue::DateTime(days) => {
                    if !days.is_finite() {
                        Err(SchemaError::ValueOutOfRange {
                            detail: "date/time values must be finite".to_owned(),
                        })
                    } else if !(OLE_DATE_MIN..OLE_DATE_MAX_EXCLUSIVE).contains(days) {
                        Err(SchemaError::ValueOutOfRange {
                            detail: format!(
                                "OLE date {days} is outside the Access-supported range \
                                 [{OLE_DATE_MIN}, {OLE_DATE_MAX_EXCLUSIVE})"
                            ),
                        })
                    } else {
                        Ok(())
                    }
                }
                _ => Err(mismatch()),
            },
            Self::Currency => match value {
                FieldValue::Currency(_) => Ok(()),
                _ => Err(mismatch()),
            },
            Self::AutoNumber(_) => Err(SchemaError::EngineAssigned { field: None }),
            Self::Boolean { .. } => match value {
                FieldValue::Boolean(_) => Ok(()),
                _ => Err(mismatch()),
            },
            Self::Binary(_) => match value {
                FieldValue::Binary(_) => Ok(()),
                _ => Err(mismatch()),
            },
        }
    }
}
