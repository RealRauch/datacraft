//! Typed field properties (step 3.1, item H10; ARCHITECTURE §7).
//!
//! The Access field-property sheet as a typed struct: defaults follow the
//! Access defaults, applicability is validated per kind.

use serde::{Deserialize, Serialize};

use crate::error::SchemaError;
use crate::field::FieldKind;

/// Indexing state of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexMode {
    /// Not indexed (Access default).
    #[default]
    None,
    /// Indexed, duplicates allowed.
    DuplicatesOk,
    /// Indexed, no duplicates (unique).
    NoDuplicates,
}

/// Access field properties with Access-compatible defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FieldProperties {
    /// Display caption (may be empty — display text, not an expression).
    pub caption: Option<String>,
    /// Input mask expression.
    pub input_mask: Option<String>,
    /// Display format expression.
    pub format: Option<String>,
    /// Default value expression (evaluated by the expression service on
    /// insert when the cell stays `Null`; step 15.2, H41).
    pub default_value: Option<String>,
    /// Validation rule expression (step 15.2, H41): only `True` accepts
    /// the row; `False` and `Null` violate.
    pub validation_rule: Option<String>,
    /// Message shown when the validation rule fails.
    pub validation_text: Option<String>,
    /// Calculated expression (step 15.2, H41): the value is engine-computed
    /// from the same row on every insert/update — never user-provided.
    #[serde(default)]
    pub calculated: Option<String>,
    /// Whether `Null` is rejected.
    pub required: bool,
    /// Whether zero-length strings are allowed (text/memo only).
    pub allow_zero_length: bool,
    /// Indexing state.
    pub indexed: IndexMode,
}

/// Expression-like properties that must not be empty when present.
const EXPRESSION_PROPERTIES: [&str; 6] = [
    "input_mask",
    "format",
    "default_value",
    "validation_rule",
    "validation_text",
    "calculated",
];

impl FieldProperties {
    /// Validates property/kind applicability with Access semantics.
    pub fn validate_for(&self, kind: &FieldKind) -> Result<(), SchemaError> {
        if self.allow_zero_length && !matches!(kind, FieldKind::Text { .. } | FieldKind::Memo) {
            return Err(SchemaError::InvalidProperty {
                detail: "allow_zero_length applies only to text and memo fields".to_owned(),
            });
        }
        if self.indexed != IndexMode::None
            && matches!(
                kind,
                FieldKind::Memo
                    | FieldKind::Binary(_)
                    | FieldKind::MultiValue(_)
                    | FieldKind::Complex
            )
        {
            return Err(SchemaError::InvalidProperty {
                detail: format!("indexing is not supported for {} fields", kind.family()),
            });
        }
        for name in EXPRESSION_PROPERTIES {
            let value = match name {
                "input_mask" => &self.input_mask,
                "format" => &self.format,
                "default_value" => &self.default_value,
                "validation_rule" => &self.validation_rule,
                "calculated" => &self.calculated,
                _ => &self.validation_text,
            };
            if value.as_ref().is_some_and(|v| v.is_empty()) {
                return Err(SchemaError::InvalidProperty {
                    detail: format!("{name} must not be an empty string — use null instead"),
                });
            }
        }
        if self.calculated.is_some() {
            if matches!(kind, FieldKind::AutoNumber(_)) {
                return Err(SchemaError::InvalidProperty {
                    detail: "calculated applies only to data fields, not AutoNumber".to_owned(),
                });
            }
            if self.default_value.is_some() {
                return Err(SchemaError::InvalidProperty {
                    detail: "a calculated field computes its own values and takes no default"
                        .to_owned(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calculated fields compute their own values: they refuse AutoNumber
    /// kinds and take no default value.
    #[test]
    fn calculated_refuses_autonumber_and_default_values() {
        let calculated = FieldProperties {
            calculated: Some("[Price]*[Qty]".to_owned()),
            ..FieldProperties::default()
        };
        let reason = calculated
            .validate_for(&FieldKind::AutoNumber(crate::AutoKind::LongInteger))
            .expect_err("calculated refuses AutoNumber");
        assert!(
            reason
                .to_string()
                .contains("calculated applies only to data fields"),
            "{reason}"
        );
        let with_default = FieldProperties {
            calculated: Some("[Price]*[Qty]".to_owned()),
            default_value: Some("0".to_owned()),
            ..FieldProperties::default()
        };
        let reason = with_default
            .validate_for(&FieldKind::Text { max_len: 50 })
            .expect_err("calculated refuses default values");
        assert!(
            reason
                .to_string()
                .contains("computes its own values and takes no default"),
            "{reason}"
        );
        assert!(
            calculated
                .validate_for(&FieldKind::Text { max_len: 50 })
                .is_ok(),
            "a plain calculated data field validates"
        );
    }
}
