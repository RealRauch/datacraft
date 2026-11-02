//! Structural schema-change helpers (moved from `database.rs` for the
//! size gate): the reducer-side application of one schema change against
//! a cloned definition, before the port rewrites the physical catalog.

use crate::port::SchemaChange;
use crate::schema::TableDef;

/// Structural application of one schema change (the port repeats the same
/// rules on the physical side). Returns the canonical field name.
pub(super) fn apply_schema_change(
    definition: &mut TableDef,
    change: &SchemaChange,
) -> Result<String, String> {
    match change {
        SchemaChange::AddField { field } => {
            if definition
                .fields
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&field.name))
            {
                return Err(format!("duplicate field {:?}", field.name));
            }
            definition.fields.push(field.clone());
            Ok(field.name.clone())
        }
        SchemaChange::DropField { field } => {
            let index = field_index(definition, field)?;
            definition.fields.remove(index);
            Ok(definition
                .fields
                .get(index.saturating_sub(1))
                .or_else(|| definition.fields.first())
                .map_or_else(|| field.clone(), |remaining| remaining.name.clone()))
        }
        SchemaChange::RenameField { field, new_name } => {
            let index = field_index(definition, field)?;
            if definition
                .fields
                .iter()
                .enumerate()
                .any(|(i, existing)| i != index && existing.name.eq_ignore_ascii_case(new_name))
            {
                return Err(format!("duplicate field {new_name:?}"));
            }
            definition.fields[index].name = new_name.clone();
            Ok(new_name.clone())
        }
        SchemaChange::SetFieldProperties { field, properties } => {
            let index = field_index(definition, field)?;
            definition.fields[index].properties = properties.clone();
            Ok(definition.fields[index].name.clone())
        }
    }
}

/// Case-insensitive field index, with a precise reason.
fn field_index(definition: &TableDef, field: &str) -> Result<usize, String> {
    definition
        .fields
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .ok_or_else(|| format!("field {field:?} not found in {:?}", definition.name))
}
