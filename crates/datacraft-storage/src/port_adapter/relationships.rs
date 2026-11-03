//! Jet relationship writes (step 32.3, item M16).
//!
//! Access keeps relationships in the `MSysRelationships` system table and
//! requires an index on both columns. DataCraft creates that system table
//! lazily, adds the parent (unique) and child (non-unique) indexes through
//! the M15 TDEF index metadata, and writes the relationship row in the same
//! writer session — a failure leaves the file untouched.

use datacraft_core::port::{PortError, Relationship, SchemaChange, StoragePort};
use datacraft_core::schema::{FieldProperties, IndexMode};

use crate::error::StorageError;
use crate::jet::tdef::JetTableDef;
use crate::jet::{JetColumn, JetType};
use crate::port_errors::{map_open, no_database};
use crate::types::StorageValue;

use super::Jet4Port;

/// Fixture shape of `MSysRelationships` (testV2000 page 5): four text
/// columns, three long counters and the relationship name.
fn msys_columns() -> Vec<JetColumn> {
    vec![
        JetColumn::new("szRelationship", JetType::Text, 0x12, 0, 0, 510),
        JetColumn::new("grbit", JetType::LongInt, 0x13, 1, 0, 4),
        JetColumn::new("ccolumn", JetType::LongInt, 0x13, 2, 0, 4),
        JetColumn::new("icolumn", JetType::LongInt, 0x13, 3, 0, 4),
        JetColumn::new("szObject", JetType::Text, 0x12, 4, 0, 510),
        JetColumn::new("szColumn", JetType::Text, 0x12, 5, 0, 510),
        JetColumn::new("szReferencedObject", JetType::Text, 0x12, 6, 0, 510),
        JetColumn::new("szReferencedColumn", JetType::Text, 0x12, 7, 0, 510),
    ]
}

pub(super) fn create(port: &mut Jet4Port, relationship: &Relationship) -> Result<(), PortError> {
    let child = port.definition_of(&relationship.table)?.clone();
    let parent = port.definition_of(&relationship.related_table)?.clone();
    let child_page = port.page_of(&relationship.table)?;
    let parent_page = port.page_of(&relationship.related_table)?;
    if port
        .relationships()?
        .iter()
        .any(|existing| existing == relationship)
    {
        return Err(PortError::WriteFailed {
            detail: format!(
                "duplicate relationship {:?}.{:?} → {:?}.{:?}",
                relationship.table,
                relationship.column,
                relationship.related_table,
                relationship.related_column
            ),
        });
    }
    let page = ensure_msys_table(port)?;
    let values = {
        let reader = port.reader.as_ref().ok_or_else(no_database)?;
        let definition = JetTableDef::decode(&reader.file, page).map_err(map_open)?;
        msys_row(&definition, relationship)
    };
    port.with_writer(|writer| {
        ensure_index(
            writer,
            parent_page,
            &parent,
            &relationship.related_column,
            IndexMode::NoDuplicates,
        )?;
        ensure_index(
            writer,
            child_page,
            &child,
            &relationship.column,
            IndexMode::DuplicatesOk,
        )?;
        writer.insert_row(page, &values)
    })?;
    Ok(())
}

pub(super) fn delete(port: &mut Jet4Port, relationship: &Relationship) -> Result<(), PortError> {
    let Some(page) = msys_page(port)? else {
        return Err(PortError::WriteFailed {
            detail: format!(
                "relationship {:?}.{:?} → {:?}.{:?} not found (no MSysRelationships table)",
                relationship.table,
                relationship.column,
                relationship.related_table,
                relationship.related_column
            ),
        });
    };
    let (definition, rows) = {
        let reader = port.reader.as_ref().ok_or_else(no_database)?;
        let definition = JetTableDef::decode(&reader.file, page).map_err(map_open)?;
        let data = reader.open_table("MSysRelationships").map_err(map_open)?;
        (definition, data.rows)
    };
    let column = |name: &str| {
        definition
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| PortError::Unsupported {
                reason: format!("MSysRelationships lacks the {name} column"),
            })
    };
    let (i_obj, i_col, i_ref_obj, i_ref_col) = (
        column("szObject")?,
        column("szColumn")?,
        column("szReferencedObject")?,
        column("szReferencedColumn")?,
    );
    let text = |row: &[StorageValue], index: usize| match row.get(index) {
        Some(StorageValue::Text(text)) => text.clone(),
        _ => String::new(),
    };
    let Some(row) = rows.iter().position(|row| {
        text(row, i_obj).eq_ignore_ascii_case(&relationship.table)
            && text(row, i_col).eq_ignore_ascii_case(&relationship.column)
            && text(row, i_ref_obj).eq_ignore_ascii_case(&relationship.related_table)
            && text(row, i_ref_col).eq_ignore_ascii_case(&relationship.related_column)
    }) else {
        return Err(PortError::WriteFailed {
            detail: format!(
                "relationship {:?}.{:?} → {:?}.{:?} not found",
                relationship.table,
                relationship.column,
                relationship.related_table,
                relationship.related_column
            ),
        });
    };
    port.with_writer(|writer| writer.delete_row(page, row))?;
    Ok(())
}

fn msys_page(port: &Jet4Port) -> Result<Option<u32>, PortError> {
    let reader = port.reader.as_ref().ok_or_else(no_database)?;
    Ok(reader
        .catalog()
        .map_err(map_open)?
        .into_iter()
        .find(|entry| entry.name.eq_ignore_ascii_case("MSysRelationships"))
        .and_then(|entry| entry.tdef_page))
}

fn ensure_msys_table(port: &mut Jet4Port) -> Result<u32, PortError> {
    if let Some(page) = msys_page(port)? {
        return Ok(page);
    }
    port.with_writer(|writer| writer.create_system_table("MSysRelationships", &msys_columns()))
}

fn ensure_index(
    writer: &mut crate::jet::Jet4Writer,
    page: u32,
    definition: &datacraft_core::schema::TableDef,
    field: &str,
    desired: IndexMode,
) -> Result<(), StorageError> {
    let Some(column) = definition
        .fields
        .iter()
        .find(|column| column.name.eq_ignore_ascii_case(field))
    else {
        return Err(StorageError::Unsupported {
            reason: format!("field {field:?} not found in {:?}", definition.name),
        });
    };
    let current = column.properties.indexed;
    if current == IndexMode::NoDuplicates || current == desired {
        return Ok(());
    }
    let properties = FieldProperties {
        indexed: desired,
        ..column.properties.clone()
    };
    writer.alter_table(
        page,
        &SchemaChange::SetFieldProperties {
            field: column.name.clone(),
            properties,
        },
    )
}

fn msys_row(definition: &JetTableDef, relationship: &Relationship) -> Vec<StorageValue> {
    definition
        .columns
        .iter()
        .map(|column| match column.name.to_ascii_lowercase().as_str() {
            "szrelationship" => StorageValue::Text(format!(
                "{}_{}_{}",
                relationship.table, relationship.column, relationship.related_table
            )),
            "szobject" => StorageValue::Text(relationship.table.clone()),
            "szcolumn" => StorageValue::Text(relationship.column.clone()),
            "szreferencedobject" => StorageValue::Text(relationship.related_table.clone()),
            "szreferencedcolumn" => StorageValue::Text(relationship.related_column.clone()),
            "grbit" | "ccolumn" | "icolumn" => StorageValue::Int(0),
            _ => StorageValue::Null,
        })
        .collect()
}
