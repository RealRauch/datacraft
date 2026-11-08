//! Referential-relationship dispatch (step 32.3, item M16).
//!
//! Relationships are a driver capability: `Yes` drivers create/delete the
//! relation and the required indexes, `ReadOnly`/`No` drivers answer the
//! precise capability reason before the port is called. Jet writes
//! `MSysRelationships` rows plus the parent/child indexes; SQLite rebuilds
//! the referencing table with a `FOREIGN KEY` clause.

use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::{DriverCapabilities, PortError, Relationship, Support};
use crate::schema::{FieldDef, FieldKind, TableDef};

impl crate::core::Core {
    /// Creates one relationship and refreshes the snapshot's relationship
    /// list and table definitions (new indexes).
    pub(crate) fn create_relationship(&mut self, relationship: Relationship) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        if db.capabilities.relationships != Support::Yes {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::NoRelationships),
                DriverCapabilities::unsupported(&db.driver, "create relationships for this source"),
            );
        }
        if let Err(reason) = validate_relationship(db, &relationship) {
            return failed(FailureCode::InvalidInput, reason);
        }
        if db
            .relationships
            .iter()
            .any(|existing| same_relationship(existing, &relationship))
        {
            return failed(
                FailureCode::Duplicate,
                format!(
                    "duplicate relationship {:?}.{:?} → {:?}.{:?}",
                    relationship.table,
                    relationship.column,
                    relationship.related_table,
                    relationship.related_column
                ),
            );
        }
        let old_tables = db.tables.clone();
        if let Err(error) = self.port.create_relationship(&relationship) {
            return failed(FailureCode::Port, error.to_string());
        }
        if let Err(reason) = self.refresh_relationships(old_tables) {
            return failed(
                FailureCode::Port,
                format!("the relationship was created but the snapshot refresh failed: {reason}"),
            );
        }
        vec![Event::RelationshipCreated { relationship }]
    }

    /// Deletes one relationship and refreshes the snapshot.
    pub(crate) fn delete_relationship(&mut self, relationship: Relationship) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        if db.capabilities.relationships != Support::Yes {
            return failed(
                FailureCode::Capability(crate::availability::DisabledReason::NoRelationships),
                DriverCapabilities::unsupported(&db.driver, "delete relationships for this source"),
            );
        }
        if !db
            .relationships
            .iter()
            .any(|existing| same_relationship(existing, &relationship))
        {
            return failed(
                FailureCode::NotFound,
                format!(
                    "relationship {:?}.{:?} → {:?}.{:?} not found",
                    relationship.table,
                    relationship.column,
                    relationship.related_table,
                    relationship.related_column
                ),
            );
        }
        let old_tables = db.tables.clone();
        if let Err(error) = self.port.delete_relationship(&relationship) {
            return failed(FailureCode::Port, error.to_string());
        }
        if let Err(reason) = self.refresh_relationships(old_tables) {
            return failed(
                FailureCode::Port,
                format!("the relationship was deleted but the snapshot refresh failed: {reason}"),
            );
        }
        vec![Event::RelationshipDeleted { relationship }]
    }

    fn refresh_relationships(&mut self, old_tables: Vec<TableDef>) -> Result<(), String> {
        let relationships = self.port.relationships().map_err(|e| e.to_string())?;
        let mut tables = self.port.tables();
        // Keep the sidecar property overlay across the driver's index
        // refresh (H45), with the driver's index metadata winning.
        crate::schema::SchemaDocument::merge_field_properties(&mut tables, &old_tables);
        if let Some(db) = self.state.database.as_mut() {
            db.tables = tables;
            db.relationships = relationships;
        }
        Ok(())
    }
}

fn validate_relationship(
    db: &crate::state::DatabaseState,
    relationship: &Relationship,
) -> Result<(), String> {
    let child = table_field(db, &relationship.table, &relationship.column)?;
    let parent = table_field(
        db,
        &relationship.related_table,
        &relationship.related_column,
    )?;
    if relationship
        .table
        .eq_ignore_ascii_case(&relationship.related_table)
        && relationship
            .column
            .eq_ignore_ascii_case(&relationship.related_column)
    {
        return Err(format!(
            "unsupported: field {:?}.{:?} cannot reference itself",
            relationship.table, relationship.column
        ));
    }
    for (field, table, label) in [
        (child, &relationship.table, "referencing"),
        (parent, &relationship.related_table, "referenced"),
    ] {
        if matches!(
            field.kind,
            FieldKind::Memo | FieldKind::Binary(_) | FieldKind::MultiValue(_) | FieldKind::Complex
        ) {
            return Err(format!(
                "unsupported: the {label} field {table:?}.{:?} cannot carry an index",
                field.name
            ));
        }
    }
    Ok(())
}

fn table_field<'a>(
    db: &'a crate::state::DatabaseState,
    table: &str,
    field: &str,
) -> Result<&'a FieldDef, String> {
    let Some(table) = db
        .tables
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
    else {
        return Err(PortError::TableNotFound(table.to_owned()).to_string());
    };
    table
        .fields
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .ok_or_else(|| format!("field {field:?} not found in {:?}", table.name))
}

fn same_relationship(left: &Relationship, right: &Relationship) -> bool {
    left.table.eq_ignore_ascii_case(&right.table)
        && left.column.eq_ignore_ascii_case(&right.column)
        && left
            .related_table
            .eq_ignore_ascii_case(&right.related_table)
        && left
            .related_column
            .eq_ignore_ascii_case(&right.related_column)
}
