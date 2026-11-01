//! Typed field properties (step 3.1, item H10; ARCHITECTURE §7).
//!
//! The Access field-property sheet as a typed struct: defaults follow the
//! Access defaults, applicability is validated per kind.

use serde::{Deserialize, Serialize};

use super::error::SchemaError;
use super::field::FieldKind;

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
    /// Default value expression (evaluated by the expression service,
    /// Ring 1 milestone M1 — persisted verbatim until then).
    pub default_value: Option<String>,
    /// Validation rule expression.
    pub validation_rule: Option<String>,
    /// Message shown when the validation rule fails.
    pub validation_text: Option<String>,
    /// Whether `Null` is rejected.
    pub required: bool,
    /// Whether zero-length strings are allowed (text/memo only).
    pub allow_zero_length: bool,
    /// Indexing state.
    pub indexed: IndexMode,
}

/// Expression-like properties that must not be empty when present.
const EXPRESSION_PROPERTIES: [&str; 5] = [
    "input_mask",
    "format",
    "default_value",
    "validation_rule",
    "validation_text",
];

impl FieldProperties {
    /// Validates property/kind applicability with Access semantics.
    pub fn validate_for(&self, kind: &FieldKind) -> Result<(), SchemaError> {
        if self.allow_zero_length && !matches!(kind, FieldKind::Text { .. } | FieldKind::Memo) {
            return Err(SchemaError::InvalidProperty {
                detail: "allow_zero_length applies only to text and memo fields".to_owned(),
            });
        }
        for name in EXPRESSION_PROPERTIES {
            let value = match name {
                "input_mask" => &self.input_mask,
                "format" => &self.format,
                "default_value" => &self.default_value,
                "validation_rule" => &self.validation_rule,
                _ => &self.validation_text,
            };
            if value.as_ref().is_some_and(|v| v.is_empty()) {
                return Err(SchemaError::InvalidProperty {
                    detail: format!("{name} must not be an empty string — use null instead"),
                });
            }
        }
        Ok(())
    }
}
