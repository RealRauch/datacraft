//! Schema error taxonomy (step 3.1, item H10).
//!
//! Errors carry enough context to locate the violated rule (LESSONS #4);
//! messages name the rule and both sides of a mismatch so headless clients
//! and the UI can render them directly.

use std::fmt;

/// Errors raised while validating field kinds, properties or values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A field kind violates its own invariants (e.g. text `max_len` 0).
    InvalidKind {
        /// Concrete rule that was violated.
        detail: String,
    },
    /// A property is not applicable to the kind or holds an empty string.
    InvalidProperty {
        /// Concrete rule that was violated.
        detail: String,
    },
    /// A value does not match the field kind's value family.
    KindMismatch {
        /// Value family the kind expects (e.g. `"currency"`).
        expected: &'static str,
        /// Value family the value actually has (e.g. `"text"`).
        found: &'static str,
    },
    /// A text value exceeds the kind's maximum length (character count).
    ValueTooLong {
        /// Maximum number of characters allowed.
        max_len: usize,
        /// Number of characters found.
        found: usize,
    },
    /// A numeric or date value is outside its kind's supported range.
    ValueOutOfRange {
        /// Concrete range and offending value.
        detail: String,
    },
    /// A required field was given `Null`.
    NullNotAllowed {
        /// Name of the field.
        field: String,
    },
    /// An empty string was given to a text field with
    /// `allow_zero_length == false`.
    EmptyStringNotAllowed {
        /// Name of the field.
        field: String,
    },
    /// A user-provided value for an AutoNumber field (engine-assigned).
    EngineAssigned {
        /// Name of the field, when known at the definition level.
        field: Option<String>,
    },
    /// Two fields of a table share a name (case-insensitive).
    DuplicateName {
        /// The duplicated name as written in the definition.
        name: String,
    },
    /// An object name is empty or whitespace-only.
    InvalidName {
        /// Concrete rule that was violated.
        detail: String,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKind { detail } => write!(f, "invalid field kind: {detail}"),
            Self::InvalidProperty { detail } => write!(f, "invalid field property: {detail}"),
            Self::KindMismatch { expected, found } => {
                write!(f, "expected a {expected} value, found a {found} value")
            }
            Self::ValueTooLong { max_len, found } => write!(
                f,
                "text value exceeds the maximum length of {max_len} characters (found {found})"
            ),
            Self::ValueOutOfRange { detail } => write!(f, "value out of range: {detail}"),
            Self::NullNotAllowed { field } => {
                write!(
                    f,
                    "field {field:?} does not allow null: the field is required"
                )
            }
            Self::EmptyStringNotAllowed { field } => {
                write!(
                    f,
                    "field {field:?} does not allow zero-length (empty) strings"
                )
            }
            Self::EngineAssigned { field: Some(field) } => write!(
                f,
                "autonumber field {field:?} is engine-assigned; user-provided values are rejected"
            ),
            Self::EngineAssigned { field: None } => {
                write!(
                    f,
                    "autonumber values are engine-assigned; user-provided values are rejected"
                )
            }
            Self::DuplicateName { name } => {
                write!(
                    f,
                    "duplicate field name {name:?}: field names are case-insensitive"
                )
            }
            Self::InvalidName { detail } => write!(f, "invalid name: {detail}"),
        }
    }
}

impl std::error::Error for SchemaError {}
