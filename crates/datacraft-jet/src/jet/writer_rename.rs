//! Table renames on the Jet4 writer (phase 72, step 72.2; item M81).
//!
//! The catalog row owns the table name (the TDEF carries none): a rename
//! rewrites the `MSysObjects.Name` cell and every `MSysRelationships`
//! object column. Row data, TDEF descriptors and page ownership stay
//! untouched. The reducer runs this inside the port's scratch writer, so
//! a failure leaves the file unchanged.

use crate::error::StorageError;
use crate::jet::catalog::{CATALOG_TDEF_PAGE, OBJECT_TYPE_TABLE};
use crate::jet::reader::Jet4TableReader;
use crate::jet::tdef::JetTableDef;
use crate::jet::writer::Jet4Writer;
use crate::types::StorageValue;

impl Jet4Writer {
    /// Renames one user table in place; returns nothing (the caller owns
    /// the canonical name).
    pub fn rename_table(&mut self, old: &str, new_name: &str) -> Result<(), StorageError> {
        self.rename_catalog_entry(old, new_name)?;
        self.rename_relationship_objects(old, new_name)
    }

    /// Rewrites the `Name` cell of the user-table catalog row.
    fn rename_catalog_entry(&mut self, old: &str, new_name: &str) -> Result<(), StorageError> {
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        let definition = JetTableDef::decode(&self.file, CATALOG_TDEF_PAGE)?;
        let name_index = catalog_column(&definition, "Name")?;
        let type_index = catalog_column(&definition, "Type")?;
        let table = reader.read_table(CATALOG_TDEF_PAGE)?;
        let position = table
            .rows
            .iter()
            .position(|row| {
                matches!(
                    row.get(type_index),
                    Some(StorageValue::Int(OBJECT_TYPE_TABLE))
                ) && matches!(
                    row.get(name_index),
                    Some(StorageValue::Text(name)) if name.eq_ignore_ascii_case(old)
                )
            })
            .ok_or_else(|| StorageError::TableNotFound(old.to_owned()))?;
        let mut values = table.rows[position].clone();
        values[name_index] = StorageValue::Text(new_name.to_owned());
        self.update_row(CATALOG_TDEF_PAGE, position, &values)
    }

    /// Rewrites both object columns of every relationship row that names
    /// the table. The relationship label (`szRelationship`) stays: it is
    /// a historical identifier, not a referential fact.
    fn rename_relationship_objects(
        &mut self,
        old: &str,
        new_name: &str,
    ) -> Result<(), StorageError> {
        let Some(page) = self.relationship_page()? else {
            return Ok(());
        };
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        let definition = JetTableDef::decode(&self.file, page)?;
        let object_index = catalog_column(&definition, "szObject")?;
        let referenced_index = catalog_column(&definition, "szReferencedObject")?;
        let rows = reader.read_table(page)?.rows;
        for (position, row) in rows.iter().enumerate() {
            let mut values = row.clone();
            let mut changed = false;
            for index in [object_index, referenced_index] {
                if let Some(StorageValue::Text(name)) = values.get_mut(index)
                    && name.eq_ignore_ascii_case(old)
                {
                    *name = new_name.to_owned();
                    changed = true;
                }
            }
            if changed {
                self.update_row(page, position, &values)?;
            }
        }
        Ok(())
    }

    /// TDEF page of `MSysRelationships`, when the catalog carries it.
    fn relationship_page(&self) -> Result<Option<u32>, StorageError> {
        let reader = Jet4TableReader::from_decrypted(self.file.clone(), None)?;
        Ok(reader
            .catalog()?
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("MSysRelationships"))
            .and_then(|entry| entry.tdef_page))
    }
}

/// Case-insensitive column index with a precise catalog-shape error.
fn catalog_column(definition: &JetTableDef, name: &str) -> Result<usize, StorageError> {
    definition
        .columns
        .iter()
        .position(|column| column.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| StorageError::Corrupt {
            context: "table rename".into(),
            detail: format!("the table definition lacks the {name:?} column"),
        })
}
