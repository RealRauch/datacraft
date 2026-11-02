//! In-memory schema alteration for `MockPort` (step 24.3, item M9).
//!
//! Mirrors the Jet writer's rules so headless core tests exercise the same
//! command vocabulary: append keeps row arity (Null), dropping removes the
//! value, renaming touches the definition only.

use std::collections::HashMap;

use crate::port::{PortError, SchemaChange};
use crate::schema::{FieldValue, TableDef};

use super::MockPort;

/// Applies one schema change to the mock's catalog and cached rows.
pub(super) fn alter(
    port: &mut MockPort,
    table: &str,
    change: SchemaChange,
) -> Result<(), PortError> {
    if let Some(detail) = &port.write_fail {
        return Err(PortError::WriteFailed {
            detail: detail.clone(),
        });
    }
    let position = port
        .tables
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(table))
        .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?;
    let definition = &mut port.tables[position];
    let canonical = definition.name.clone();
    match change {
        SchemaChange::AddField { field } => {
            if definition
                .fields
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&field.name))
            {
                return Err(PortError::WriteFailed {
                    detail: format!("duplicate field {:?}", field.name),
                });
            }
            definition.fields.push(field);
            if let Some(rows) = rows_of(&mut port.rows, &canonical) {
                for row in rows {
                    row.push(FieldValue::Null);
                }
            }
        }
        SchemaChange::DropField { field } => {
            let index = field_position(definition, &field)?;
            definition.fields.remove(index);
            if let Some(rows) = rows_of(&mut port.rows, &canonical) {
                for row in rows {
                    if index < row.len() {
                        row.remove(index);
                    }
                }
            }
        }
        SchemaChange::RenameField { field, new_name } => {
            let index = field_position(definition, &field)?;
            if definition
                .fields
                .iter()
                .enumerate()
                .any(|(i, existing)| i != index && existing.name.eq_ignore_ascii_case(&new_name))
            {
                return Err(PortError::WriteFailed {
                    detail: format!("duplicate field {new_name:?}"),
                });
            }
            definition.fields[index].name = new_name;
        }
        SchemaChange::SetFieldProperties { field, properties } => {
            let index = field_position(definition, &field)?;
            definition.fields[index].properties = properties;
        }
    }
    Ok(())
}

/// Row storage of one table (case-insensitive key), when present.
fn rows_of<'a>(
    rows: &'a mut HashMap<String, Vec<Vec<FieldValue>>>,
    table: &str,
) -> Option<&'a mut Vec<Vec<FieldValue>>> {
    rows.iter_mut()
        .find(|(key, _)| key.eq_ignore_ascii_case(table))
        .map(|(_, rows)| rows)
}

/// Case-insensitive field index with a precise write failure.
fn field_position(definition: &TableDef, field: &str) -> Result<usize, PortError> {
    definition
        .fields
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .ok_or_else(|| PortError::WriteFailed {
            detail: format!("field {field:?} not found in {:?}", definition.name),
        })
}
