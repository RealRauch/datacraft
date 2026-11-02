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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, FieldKind, FieldProperties, NumericKind};

    fn mock_port() -> MockPort {
        let mut port = MockPort::with_tables(vec![TableDef::new(
            "T",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        )]);
        port.rows
            .insert("T".to_owned(), vec![vec![FieldValue::Integer(1)]]);
        port
    }

    #[test]
    fn write_failures_and_unknown_tables_are_precise() {
        let mut port = mock_port();
        port.write_fail = Some("disk full".to_owned());
        let error = alter(
            &mut port,
            "T",
            SchemaChange::DropField {
                field: "id".to_owned(),
            },
        )
        .expect_err("write failure");
        assert!(error.to_string().contains("disk full"), "{error}");

        let mut port = mock_port();
        let error = alter(
            &mut port,
            "missing",
            SchemaChange::DropField {
                field: "id".to_owned(),
            },
        )
        .expect_err("unknown table");
        assert!(matches!(error, PortError::TableNotFound(_)), "{error:?}");
    }

    #[test]
    fn duplicate_and_unknown_fields_are_rejected() {
        let mut port = mock_port();
        let error = alter(
            &mut port,
            "T",
            SchemaChange::AddField {
                field: FieldDef::new("ID", FieldKind::Text { max_len: 2 }),
            },
        )
        .expect_err("duplicate add");
        assert!(error.to_string().contains("duplicate"), "{error}");

        let error = alter(
            &mut port,
            "T",
            SchemaChange::DropField {
                field: "missing".to_owned(),
            },
        )
        .expect_err("unknown field");
        assert!(error.to_string().contains("not found"), "{error}");
    }

    #[test]
    fn rename_and_properties_round_trip_without_touching_rows() {
        let mut port = mock_port();
        alter(
            &mut port,
            "T",
            SchemaChange::RenameField {
                field: "id".to_owned(),
                new_name: "key".to_owned(),
            },
        )
        .expect("rename");
        assert_eq!(port.tables[0].fields[0].name, "key");
        alter(
            &mut port,
            "T",
            SchemaChange::SetFieldProperties {
                field: "key".to_owned(),
                properties: FieldProperties {
                    required: true,
                    ..Default::default()
                },
            },
        )
        .expect("properties");
        assert!(port.tables[0].fields[0].properties.required);
        assert_eq!(port.rows["T"], [vec![FieldValue::Integer(1)]]);
    }

    #[test]
    fn adding_to_a_table_without_row_storage_still_works() {
        let mut port = MockPort::with_tables(vec![TableDef::new(
            "Empty",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        )]);
        alter(
            &mut port,
            "Empty",
            SchemaChange::AddField {
                field: FieldDef::new("note", FieldKind::Text { max_len: 3 }),
            },
        )
        .expect("add on empty storage");
        assert_eq!(port.tables[0].fields.len(), 2);
    }
}
