//! Row read/write surface of the Jet port (moved from `port_adapter.rs`
//! for the file-size gate): `read_rows`, the single-row writes and the
//! atomic `write_row_ops` batch. The port's trait impl calls these as
//! thin wrappers — a trait impl cannot be split across modules, so the
//! bodies live here as free functions over the port.

use std::collections::HashMap;

use datacraft_core::port::{PortError, RowRange, RowWriteOp};
use datacraft_core::schema::{FieldDef, FieldValue, IndexMode};

use super::{Jet4Port, map_open, no_database, port_write};
use crate::jet::complex::{self, FlatRows};
use crate::jet::tdef::JetTableDef;
use crate::mapping::to_field_value;

/// One validated operation of a batch write (domain values mapped to the
/// storage vocabulary before any byte is written).
enum PreparedOp {
    Insert(Vec<crate::types::StorageValue>),
    Update {
        row: usize,
        values: Vec<crate::types::StorageValue>,
    },
    Delete {
        row: usize,
    },
}

pub(super) fn insert_row(
    port: &mut Jet4Port,
    table: &str,
    values: &[FieldValue],
) -> Result<(), PortError> {
    if port.reader.is_none() {
        return Err(no_database());
    }
    let Some(page) = port
        .pages
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(table))
        .map(|(_, page)| *page)
    else {
        return Err(PortError::TableNotFound(table.to_owned()));
    };
    let Some(def) = port
        .tables
        .iter()
        .find(|def| def.name.eq_ignore_ascii_case(table))
    else {
        return Err(PortError::TableNotFound(table.to_owned()));
    };
    check_unique(port, table, def, values, None)?;
    let stored = port_write::to_storage_values(&def.fields, values)?;
    port.with_writer(|writer| writer.insert_row(page, &stored))?;
    Ok(())
}

pub(super) fn read_rows(
    port: &Jet4Port,
    table: &str,
    range: RowRange,
) -> Result<Vec<Vec<FieldValue>>, PortError> {
    let reader = port.reader.as_ref().ok_or_else(|| PortError::Unsupported {
        reason: "no database is open".to_owned(),
    })?;
    let page = port.page_of(table)?;
    if port.complex.is_flat_page(page) {
        // Backing tables are an implementation detail, not user tables.
        return Err(PortError::TableNotFound(table.to_owned()));
    }
    let data = reader.open_table(table).map_err(|e| match e {
        crate::error::StorageError::TableNotFound(name) => PortError::TableNotFound(name),
        other => map_open(other),
    })?;
    let def = JetTableDef::decode(&reader.file, page).map_err(map_open)?;
    let (start, end) = datacraft_core::port::window_slice(table, range, data.rows.len())?;
    let mut flats: HashMap<u32, FlatRows> = HashMap::new();
    let mut rows = Vec::with_capacity(end - start);
    for row in &data.rows[start..end] {
        let mut values = Vec::with_capacity(row.len());
        for (column, value) in def.columns.iter().zip(row) {
            let Some(mapping) = port.complex.column(page, &column.name) else {
                values.push(to_field_value(value));
                continue;
            };
            if let std::collections::hash_map::Entry::Vacant(entry) = flats.entry(mapping.flat_tdef)
            {
                entry.insert(FlatRows::read(reader, mapping).map_err(map_open)?);
            }
            let flat = flats.get(&mapping.flat_tdef).expect("inserted above");
            values.push(complex::resolve_cell(value, flat).map_err(map_open)?);
        }
        rows.push(values);
    }
    Ok(rows)
}

pub(super) fn update_row(
    port: &mut Jet4Port,
    table: &str,
    row: usize,
    values: &[FieldValue],
) -> Result<(), PortError> {
    if port.reader.is_none() {
        return Err(no_database());
    }
    let page = port.page_of(table)?;
    let stored = {
        let definition = port.definition_of(table)?.clone();
        check_unique(port, table, &definition, values, Some(row))?;
        port_write::to_storage_values_for_update(&definition.fields, values)?
    };
    port.with_writer(|writer| writer.update_row(page, row, &stored))?;
    Ok(())
}

pub(super) fn delete_row(port: &mut Jet4Port, table: &str, row: usize) -> Result<(), PortError> {
    if port.reader.is_none() {
        return Err(no_database());
    }
    let page = port.page_of(table)?;
    port.with_writer(|writer| writer.delete_row(page, row))?;
    Ok(())
}

pub(super) fn write_row_ops(
    port: &mut Jet4Port,
    table: &str,
    ops: &[datacraft_core::port::RowWriteOp],
) -> Result<(), PortError> {
    if port.reader.is_none() {
        return Err(no_database());
    }
    let page = port.page_of(table)?;
    let definition = port.definition_of(table)?.clone();
    check_batch_unique(port, table, &definition, ops)?;
    // Validate and map every operation before a single byte is written.
    let mut prepared = Vec::with_capacity(ops.len());
    for op in ops {
        prepared.push(match op {
            datacraft_core::port::RowWriteOp::Insert(values) => {
                PreparedOp::Insert(port_write::to_storage_values(&definition.fields, values)?)
            }
            datacraft_core::port::RowWriteOp::Update { row, values } => PreparedOp::Update {
                row: *row,
                values: port_write::to_storage_values_for_update(&definition.fields, values)?,
            },
            datacraft_core::port::RowWriteOp::Delete { row } => PreparedOp::Delete { row: *row },
        });
    }
    // One writer session: the file is persisted only when every
    // operation succeeded — the batch is atomic on this backend.
    port.with_writer(|writer| {
        for op in &prepared {
            match op {
                PreparedOp::Insert(values) => writer.insert_row(page, values)?,
                PreparedOp::Update { row, values } => writer.update_row(page, *row, values)?,
                PreparedOp::Delete { row } => writer.delete_row(page, *row)?,
            }
        }
        Ok(())
    })?;
    Ok(())
}

/// Fields carrying a `NoDuplicates` index.
fn unique_columns(definition: &datacraft_core::schema::TableDef) -> Vec<(usize, &FieldDef)> {
    definition
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.properties.indexed == IndexMode::NoDuplicates)
        .collect()
}

/// Rejects a non-null duplicate value on any unique index. `skip` ignores
/// the row being updated; NULLs never collide (Access `Ignore Nulls`).
fn check_unique(
    port: &Jet4Port,
    table: &str,
    definition: &datacraft_core::schema::TableDef,
    values: &[FieldValue],
    skip: Option<usize>,
) -> Result<(), PortError> {
    let unique = unique_columns(definition);
    if unique.is_empty() {
        return Ok(());
    }
    let rows = read_rows(
        port,
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )?;
    for (row_index, row) in rows.iter().enumerate() {
        if skip == Some(row_index) {
            continue;
        }
        check_row_against(definition, &unique, values, row)?;
    }
    Ok(())
}

fn check_row_against(
    definition: &datacraft_core::schema::TableDef,
    unique: &[(usize, &FieldDef)],
    candidate: &[FieldValue],
    existing: &[FieldValue],
) -> Result<(), PortError> {
    for (column, field) in unique {
        let new = candidate.get(*column).unwrap_or(&FieldValue::Null);
        let old = existing.get(*column).unwrap_or(&FieldValue::Null);
        if !matches!(new, FieldValue::Null) && !matches!(old, FieldValue::Null) && new == old {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "table {:?}: value {:?} violates the unique index on field {:?}",
                    definition.name, new, field.name
                ),
            });
        }
    }
    Ok(())
}

/// Simulates the batch over the current rows and rejects any duplicate a
/// single operation would introduce (the batch is atomic on this backend).
fn check_batch_unique(
    port: &Jet4Port,
    table: &str,
    definition: &datacraft_core::schema::TableDef,
    ops: &[RowWriteOp],
) -> Result<(), PortError> {
    let unique = unique_columns(definition);
    if unique.is_empty() {
        return Ok(());
    }
    let mut rows = read_rows(
        port,
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )?;
    for op in ops {
        match op {
            RowWriteOp::Insert(values) => {
                for row in &rows {
                    check_row_against(definition, &unique, values, row)?;
                }
                rows.push(values.clone());
            }
            RowWriteOp::Update { row, values } => {
                for (index, existing) in rows.iter().enumerate() {
                    if index == *row {
                        continue;
                    }
                    check_row_against(definition, &unique, values, existing)?;
                }
                if let Some(slot) = rows.get_mut(*row) {
                    *slot = values.clone();
                }
            }
            RowWriteOp::Delete { row } => {
                if *row < rows.len() {
                    rows.remove(*row);
                }
            }
        }
    }
    Ok(())
}
