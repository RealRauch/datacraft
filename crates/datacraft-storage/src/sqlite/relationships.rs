//! SQLite relationship creation/deletion (step 32.3, item M16).
//!
//! SQLite cannot add a foreign key to an existing table, so the referencing
//! table is rebuilt inside one transaction (create with the FK clauses,
//! copy rows, drop, rename, recreate indexes) with `foreign_keys` disabled.
//! The parent column gets a unique index, the child column a plain index;
//! existing rows are checked for orphans before the rebuild.

use datacraft_core::port::{PortError, Relationship};
use datacraft_core::schema::{IndexMode, TableDef};

use super::SqlitePort;
use super::mapping::{create_sql_named, foreign_keys, no_database, quote_ident, read_only};

pub(super) fn create(port: &mut SqlitePort, relationship: &Relationship) -> Result<(), PortError> {
    let connection = port.conn.as_ref().ok_or_else(no_database)?;
    if !port.writable {
        return Err(read_only());
    }
    let definition = table(port, &relationship.table)?;
    let parent = table(port, &relationship.related_table)?;
    field(&definition, &relationship.column)?;
    field(&parent, &relationship.related_column)?;
    let mut fks = foreign_keys(connection, &definition)?;
    if fks.iter().any(|fk| fk == relationship) {
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
    fks.push(relationship.clone());
    check_orphans(connection, relationship)?;
    rebuild(port, &definition, &fks)?;
    set_indexed(
        &mut port.tables,
        &relationship.related_table,
        &relationship.related_column,
        IndexMode::NoDuplicates,
    );
    set_indexed(
        &mut port.tables,
        &relationship.table,
        &relationship.column,
        IndexMode::DuplicatesOk,
    );
    Ok(())
}

pub(super) fn delete(port: &mut SqlitePort, relationship: &Relationship) -> Result<(), PortError> {
    let connection = port.conn.as_ref().ok_or_else(no_database)?;
    if !port.writable {
        return Err(read_only());
    }
    let definition = table(port, &relationship.table)?;
    let mut fks = foreign_keys(connection, &definition)?;
    let Some(position) = fks.iter().position(|fk| fk == relationship) else {
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
    fks.remove(position);
    rebuild(port, &definition, &fks)
}

fn table(port: &SqlitePort, name: &str) -> Result<TableDef, PortError> {
    port.tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(name))
        .cloned()
        .ok_or_else(|| PortError::TableNotFound(name.to_owned()))
}

fn field(table: &TableDef, name: &str) -> Result<(), PortError> {
    if table
        .fields
        .iter()
        .any(|field| field.name.eq_ignore_ascii_case(name))
    {
        Ok(())
    } else {
        Err(PortError::Unsupported {
            reason: format!("field {name:?} not found in {:?}", table.name),
        })
    }
}

fn check_orphans(
    connection: &rusqlite::Connection,
    relationship: &Relationship,
) -> Result<(), PortError> {
    let sql = format!(
        "SELECT COUNT(*) FROM {child} c LEFT JOIN {parent} p ON c.{column} = p.{related} \
         WHERE c.{column} IS NOT NULL AND p.{related} IS NULL",
        child = quote_ident(&relationship.table),
        parent = quote_ident(&relationship.related_table),
        column = quote_ident(&relationship.column),
        related = quote_ident(&relationship.related_column),
    );
    let count: i64 = connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|e| PortError::WriteFailed {
            detail: format!("checking existing relationship rows failed: {e}"),
        })?;
    if count > 0 {
        return Err(PortError::WriteFailed {
            detail: format!(
                "{count} existing row(s) of {:?} reference a missing {:?}.{:?} — the \
                 relationship cannot be created",
                relationship.table, relationship.related_table, relationship.related_column
            ),
        });
    }
    Ok(())
}

fn rebuild(
    port: &mut SqlitePort,
    definition: &TableDef,
    fks: &[Relationship],
) -> Result<(), PortError> {
    let connection = port.conn.as_ref().ok_or_else(no_database)?;
    let temp = format!("{}__datacraft", definition.name);
    let create = create_sql_named(definition, &temp, fks)?;
    let columns = definition
        .fields
        .iter()
        .map(|field| quote_ident(&field.name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut batch = String::new();
    batch.push_str("BEGIN;\n");
    batch.push_str(&create);
    batch.push_str(";\n");
    batch.push_str(&format!(
        "INSERT INTO {} ({columns}) SELECT {columns} FROM {};\n",
        quote_ident(&temp),
        quote_ident(&definition.name)
    ));
    batch.push_str(&format!(
        "DROP TABLE {};\nALTER TABLE {} RENAME TO {};\n",
        quote_ident(&definition.name),
        quote_ident(&temp),
        quote_ident(&definition.name)
    ));
    for field in &definition.fields {
        if field.properties.indexed == IndexMode::None {
            continue;
        }
        batch.push_str(&index_sql(
            definition,
            &field.name,
            field.properties.indexed,
        ));
    }
    for fk in fks {
        if fk.table.eq_ignore_ascii_case(&definition.name)
            && !definition
                .fields
                .iter()
                .any(|field| field.name.eq_ignore_ascii_case(&fk.column))
        {
            continue;
        }
        if fk.table.eq_ignore_ascii_case(&definition.name) {
            let indexed = definition
                .fields
                .iter()
                .find(|field| field.name.eq_ignore_ascii_case(&fk.column))
                .is_some_and(|field| field.properties.indexed != IndexMode::None);
            if !indexed {
                batch.push_str(&index_sql(definition, &fk.column, IndexMode::DuplicatesOk));
            }
        }
        batch.push_str(&format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {} ON {} ({});\n",
            quote_ident(&format!("uq_{}_{}", fk.related_table, fk.related_column)),
            quote_ident(&fk.related_table),
            quote_ident(&fk.related_column)
        ));
    }
    batch.push_str("COMMIT;");
    connection
        .execute_batch("PRAGMA foreign_keys=OFF;")
        .map_err(|e| PortError::WriteFailed {
            detail: format!("disabling foreign keys failed: {e}"),
        })?;
    let result = connection.execute_batch(&batch);
    let _ = connection.execute_batch("PRAGMA foreign_keys=ON;");
    if let Err(error) = result {
        let _ = connection.execute_batch("ROLLBACK;");
        return Err(PortError::WriteFailed {
            detail: format!(
                "rebuilding table {:?} for the relationship failed: {error}",
                definition.name
            ),
        });
    }
    Ok(())
}

fn index_sql(definition: &TableDef, column: &str, mode: IndexMode) -> String {
    let unique = if mode == IndexMode::NoDuplicates {
        "UNIQUE "
    } else {
        ""
    };
    format!(
        "CREATE {unique}INDEX {} ON {} ({});\n",
        quote_ident(&format!("idx_{}_{}", definition.name, column)),
        quote_ident(&definition.name),
        quote_ident(column)
    )
}

fn set_indexed(tables: &mut [TableDef], table: &str, column: &str, mode: IndexMode) {
    let Some(table) = tables
        .iter_mut()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
    else {
        return;
    };
    let Some(field) = table
        .fields
        .iter_mut()
        .find(|field| field.name.eq_ignore_ascii_case(column))
    else {
        return;
    };
    if field.properties.indexed == IndexMode::None || mode == IndexMode::NoDuplicates {
        field.properties.indexed = mode;
    }
}
