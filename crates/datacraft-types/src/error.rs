//! Schema error taxonomy (step 3.1, item H10).
//!
//! Errors carry enough context to locate the violated rule (LESSONS #4);
//! messages name the rule and both sides of a mismatch so headless clients
//! and the UI can render them directly.

use std::fmt;

/// Errors raised while validating field kinds, properties or values.
#[derive(Debug)]
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
    /// A schema document carries a version this DataCraft cannot read.
    UnsupportedVersion {
        /// Version found in the document.
        found: u64,
        /// Highest version this DataCraft supports.
        supported: u64,
    },
    /// The document is not well-formed JSON or misses required fields.
    Parse {
        /// Underlying parser detail.
        detail: String,
    },
    /// Reading or writing the sidecar file failed.
    Io(std::io::Error),
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
                    "duplicate name {name:?}: object names are case-insensitive"
                )
            }
            Self::InvalidName { detail } => write!(f, "invalid name: {detail}"),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "schema document version {found} is not supported — this DataCraft \
                 reads and writes versions up to {supported}"
            ),
            Self::Parse { detail } => write!(f, "malformed schema document: {detail}"),
            Self::Io(e) => write!(f, "schema document I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for SchemaError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the I/O variant chains an underlying error source; the rest are
    /// self-contained (contract for error handling in clients).
    #[test]
    fn io_errors_chain_their_source() {
        let io: SchemaError = std::io::Error::other("disk full").into();
        assert!(std::error::Error::source(&io).is_some());
        assert!(io.to_string().contains("disk full"));

        let parse = SchemaError::Parse {
            detail: "bad".to_owned(),
        };
        assert!(std::error::Error::source(&parse).is_none());
        assert!(parse.to_string().contains("bad"));
    }
}
