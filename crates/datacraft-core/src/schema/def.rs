//! Field and table definitions (step 3.1, item H10).
//!
//! `FieldDef` and `TableDef` compose kinds and properties into the domain
//! schema model; the versioned sidecar document of step 3.2 (ADR-1)
//! serializes tables of these definitions.

use serde::{Deserialize, Serialize};

use super::error::SchemaError;
use super::field::FieldKind;
use super::properties::FieldProperties;
use super::value::FieldValue;

/// One field of a table: name, kind and properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name (unique within its table, case-insensitive).
    pub name: String,
    /// Logical Access type.
    pub kind: FieldKind,
    /// Access field properties.
    pub properties: FieldProperties,
}

impl FieldDef {
    /// Creates a field definition with Access-default properties.
    pub fn new(name: impl Into<String>, kind: FieldKind) -> Self {
        Self {
            name: name.into(),
            kind,
            properties: FieldProperties::default(),
        }
    }

    /// Validates name, kind invariants and property applicability.
    pub fn validate(&self) -> Result<(), SchemaError> {
        validate_object_name(&self.name)?;
        self.kind.validate()?;
        self.properties.validate_for(&self.kind)?;
        Ok(())
    }

    /// Validates a value against this field: required-ness and zero-length
    /// rules first, then the kind's own value rules. AutoNumber rejections
    /// gain the field name for precise diagnostics.
    pub fn validate_value(&self, value: &FieldValue) -> Result<(), SchemaError> {
        match self.kind.validate_value(value) {
            Err(SchemaError::EngineAssigned { field: None }) => Err(SchemaError::EngineAssigned {
                field: Some(self.name.clone()),
            }),
            other => {
                if matches!(value, FieldValue::Null) {
                    if self.properties.required {
                        return Err(SchemaError::NullNotAllowed {
                            field: self.name.clone(),
                        });
                    }
                    return Ok(());
                }
                if let FieldValue::Text(s) = value
                    && s.is_empty()
                    && !self.properties.allow_zero_length
                    && matches!(self.kind, FieldKind::Text { .. } | FieldKind::Memo)
                {
                    return Err(SchemaError::EmptyStringNotAllowed {
                        field: self.name.clone(),
                    });
                }
                other
            }
        }
    }
}

/// Definition of one table: name plus ordered field definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDef {
    /// Table name (unique in its database, case-insensitive).
    pub name: String,
    /// Field definitions in storage order.
    pub fields: Vec<FieldDef>,
}

impl TableDef {
    /// Access-style default for a new table (step 11.3): an `ID` long
    /// integer plus `Field1` short text. AutoNumber is deliberately absent
    /// until the Jet writer maintains its counter (Phase 11.10).
    pub fn access_default(name: impl Into<String>) -> Self {
        Self::new(
            name,
            vec![
                FieldDef::new("ID", FieldKind::Number(super::NumericKind::LongInteger)),
                FieldDef::new("Field1", FieldKind::Text { max_len: 255 }),
            ],
        )
    }

    /// Creates a table definition.
    pub fn new(name: impl Into<String>, fields: Vec<FieldDef>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }

    /// Validates the table name, every field and field-name uniqueness
    /// (Access compares object names case-insensitively).
    pub fn validate(&self) -> Result<(), SchemaError> {
        validate_object_name(&self.name)?;
        let mut seen: Vec<String> = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            field.validate()?;
            let key = field.name.to_lowercase();
            if seen.contains(&key) {
                return Err(SchemaError::DuplicateName {
                    name: field.name.clone(),
                });
            }
            seen.push(key);
        }
        Ok(())
    }
}

/// Object names must not be empty or whitespace-only (Access enforces this
/// in the UI; we enforce it at the model boundary).
pub(crate) fn validate_object_name(name: &str) -> Result<(), SchemaError> {
    if name.trim().is_empty() {
        Err(SchemaError::InvalidName {
            detail: "names must not be empty or whitespace-only".to_owned(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::NumericKind;

    use super::{FieldKind, TableDef};

    /// The Access-style default table is one ID long integer plus one
    /// short-text field — the new-table flow's shape.
    #[test]
    fn access_default_is_id_plus_one_text_field() {
        let table = TableDef::access_default("New");
        assert_eq!(table.name, "New");
        assert_eq!(table.fields.len(), 2);
        assert_eq!(table.fields[0].name, "ID");
        assert_eq!(
            table.fields[0].kind,
            FieldKind::Number(NumericKind::LongInteger)
        );
        assert_eq!(table.fields[1].name, "Field1");
        assert_eq!(table.fields[1].kind, FieldKind::Text { max_len: 255 });
    }
}
