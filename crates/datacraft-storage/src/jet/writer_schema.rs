//! Design-time schema editing (step 24.3, item M9) — ALTER TABLE on the
//! Jet4 writer.
//!
//! A change is applied to a scratch copy first, so a failure (unsupported
//! row shape, TDEF overflow) leaves the writer's file untouched. Supported
//! changes: append a nullable field (`Null` for existing rows), drop a
//! field (rows lose the value), rename a field (TDEF only) and replace the
//! `required` property (the nullable flag). Storage-shape changes answer a
//! precise `Unsupported`.

use datacraft_core::port::SchemaChange;

use crate::error::StorageError;
use crate::jet::encode::{build_table_def, encode_tdef};
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::{JetColumn, JetTableDef, flags};
use crate::jet::writer::Jet4Writer;
use crate::port_write;
use crate::types::StorageValue;

/// How existing row values change.
#[derive(Clone, Copy)]
enum RowChange {
    /// Row bytes stay valid (rename, property flag changes).
    Identity,
    /// Appended column: every row gains a Null.
    AddNull,
    /// Dropped column: every row loses the value at `index`.
    Remove(usize),
}

impl Jet4Writer {
    /// Applies one schema change atomically (scratch writer first).
    pub fn alter_table(
        &mut self,
        tdef_page: u32,
        change: &SchemaChange,
    ) -> Result<(), StorageError> {
        let mut trial = Jet4Writer::new(self.file.clone())?;
        trial.alter_table_in_place(tdef_page, change)?;
        self.file = trial.file;
        self.header = trial.header;
        Ok(())
    }

    fn alter_table_in_place(
        &mut self,
        tdef_page: u32,
        change: &SchemaChange,
    ) -> Result<(), StorageError> {
        self.ensure_plausible_tdef(tdef_page)?;
        let old = JetTableDef::decode(&self.file, tdef_page)?;
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        let old_rows = reader.read_table(tdef_page)?.rows;
        let mut columns = old.columns.clone();
        let row_change = match change {
            SchemaChange::AddField { field } => {
                if old
                    .columns
                    .iter()
                    .any(|column| column.name.eq_ignore_ascii_case(&field.name))
                {
                    return Err(StorageError::SchemaViolation {
                        table: "alter table".to_owned(),
                        detail: format!("duplicate field {:?}", field.name),
                    });
                }
                if field.properties.required && !old_rows.is_empty() {
                    return Err(StorageError::Unsupported {
                        reason: format!(
                            "adding required field {:?} to a table with rows is not supported — \
                             existing rows would need a default value",
                            field.name
                        ),
                    });
                }
                let template = datacraft_core::schema::TableDef::new("T", vec![field.clone()]);
                let mut column = port_write::table_to_columns(&template)
                    .map_err(|error| StorageError::Unsupported {
                        reason: error.to_string(),
                    })?
                    .remove(0);
                column.column_id = columns
                    .iter()
                    .map(|existing| existing.column_id)
                    .max()
                    .map_or(0, |max| max.saturating_add(1));
                columns.push(column);
                RowChange::AddNull
            }
            SchemaChange::DropField { field } => {
                let index = column_index(&columns, field)?;
                if columns.len() == 1 {
                    return Err(StorageError::Unsupported {
                        reason: "a table must keep at least one field".to_owned(),
                    });
                }
                columns.remove(index);
                // Rows store a prefix of the column-id space (decoder rule);
                // dropping a column therefore renumbers the remaining ids
                // densely so every existing row stays within the prefix.
                for (id, column) in columns.iter_mut().enumerate() {
                    column.column_id = id as u16;
                }
                RowChange::Remove(index)
            }
            SchemaChange::RenameField { field, new_name } => {
                let index = column_index(&columns, field)?;
                if columns
                    .iter()
                    .enumerate()
                    .any(|(i, column)| i != index && column.name.eq_ignore_ascii_case(new_name))
                {
                    return Err(StorageError::SchemaViolation {
                        table: "alter table".to_owned(),
                        detail: format!("duplicate field {new_name:?}"),
                    });
                }
                columns[index].name = new_name.clone();
                RowChange::Identity
            }
            SchemaChange::SetFieldProperties { field, properties } => {
                let index = column_index(&columns, field)?;
                if !only_required_changed(properties) {
                    return Err(StorageError::Unsupported {
                        reason: "only the required property is stored in the TDEF; other \
                                 property changes are not writable"
                            .to_owned(),
                    });
                }
                let column = &mut columns[index];
                if properties.required {
                    column.flags &= !flags::NULLABLE;
                } else {
                    column.flags |= flags::NULLABLE;
                }
                RowChange::Identity
            }
        };

        let mut updated = build_table_def(
            old.table_type,
            old.num_rows,
            &columns,
            old.used_pages,
            old.free_pages,
        );
        updated.autonumber = old.autonumber;
        let block = encode_tdef(&updated);
        let base = self.header.page_offset(tdef_page)? as usize;
        let page_size = self.header.page_size;
        if block.len() + 8 > page_size {
            return Err(StorageError::Unsupported {
                reason: format!(
                    "the altered table definition needs {} bytes and does not fit TDEF page {}",
                    block.len(),
                    tdef_page
                ),
            });
        }
        self.file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
        // Clear the stale tail (the decoder is length-driven; the fill keeps
        // freed descriptor bytes from reading as garbage).
        let tail = base + 8 + block.len()..base + page_size;
        self.file[tail].fill(0x00);

        if !matches!(row_change, RowChange::Identity) {
            for (index, row) in old_rows.iter().enumerate() {
                let values = match row_change {
                    RowChange::AddNull => {
                        let mut values = row.clone();
                        values.push(StorageValue::Null);
                        values
                    }
                    RowChange::Remove(drop) => {
                        let mut values = row.clone();
                        if drop < values.len() {
                            values.remove(drop);
                        }
                        values
                    }
                    RowChange::Identity => row.clone(),
                };
                self.update_row(tdef_page, index, &values)?;
            }
        }
        Ok(())
    }
}

/// Case-insensitive column index, precise on miss.
fn column_index(columns: &[JetColumn], field: &str) -> Result<usize, StorageError> {
    columns
        .iter()
        .position(|column| column.name.eq_ignore_ascii_case(field))
        .ok_or_else(|| StorageError::SchemaViolation {
            table: "alter table".to_owned(),
            detail: format!("field {field:?} not found"),
        })
}

/// Whether the property set carries nothing the TDEF cannot represent.
fn only_required_changed(properties: &datacraft_core::schema::FieldProperties) -> bool {
    datacraft_core::schema::FieldProperties {
        required: properties.required,
        ..Default::default()
    } == *properties
}
