//! SQLite backend behind the core [`StoragePort`] (steps 13.1/13.2, H35/H36).
//!
//! SQLite is the second source family next to Jet/ACE: it is opened by file
//! magic, catalogued through `sqlite_master`, and mapped onto the same
//! domain vocabulary. The affinity table (SQLite declared type -> Access
//! field kind) is documented in `docs/FORMAT-JET.md` §"SQLite backend".

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, OpenedDatabase, PortError, Relationship, RowRange, StoragePort,
};
use datacraft_core::schema::{FieldKind, FieldValue, TableDef};
use rusqlite::Connection;

/// SQLite backend over one open database file.
#[derive(Debug, Default)]
pub struct SqlitePort {
    conn: Option<Connection>,
    tables: Vec<TableDef>,
    path: Option<PathBuf>,
}

impl SqlitePort {
    /// Creates an unattached backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// The open file's path (tests and diagnostics).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl StoragePort for SqlitePort {
    fn open(&mut self, path: &Path) -> Result<OpenedDatabase, PortError> {
        read_magic(path)?;
        let conn = open_writable(path)?;
        let tables = collect_tables(&conn)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.conn = Some(conn);
        self.tables = tables;
        self.path = Some(path.to_path_buf());
        Ok(OpenedDatabase {
            name,
            format: DatabaseFormat::Sqlite,
        })
    }

    fn close(&mut self) {
        self.conn = None;
        self.tables.clear();
        self.path = None;
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        definition.validate().map_err(|e| PortError::WriteFailed {
            detail: e.to_string(),
        })?;
        let sql = create_sql(definition)?;
        conn.execute_batch(&sql)
            .map_err(|e| PortError::WriteFailed {
                detail: format!("creating table {:?} failed: {e}", definition.name),
            })?;
        self.tables.push(definition.clone());
        Ok(())
    }

    fn drop_table(&mut self, name: &str) -> Result<String, PortError> {
        let Some(pos) = self
            .tables
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))
        else {
            return Err(PortError::TableNotFound(name.to_owned()));
        };
        let canonical = self.tables[pos].name.clone();
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        conn.execute(&format!("DROP TABLE {}", quote_ident(&canonical)), [])
            .map_err(|e| PortError::WriteFailed {
                detail: format!("dropping table {canonical:?} failed: {e}"),
            })?;
        self.tables.remove(pos);
        Ok(canonical)
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        let def = self
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?
            .clone();
        if values.len() != def.fields.len() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "insert into {:?}: expected {} values, found {}",
                    def.name,
                    def.fields.len(),
                    values.len()
                ),
            });
        }
        let mut columns = Vec::new();
        let mut params = Vec::new();
        for (field, value) in def.fields.iter().zip(values) {
            if matches!(field.kind, FieldKind::AutoNumber(_)) {
                if matches!(value, FieldValue::Null) {
                    continue;
                }
                return Err(PortError::WriteFailed {
                    detail: format!(
                        "autonumber field {:?} is engine-assigned; user-provided values are rejected",
                        field.name
                    ),
                });
            }
            params.push(sql_value(&field.kind, value));
            columns.push(quote_ident(&field.name));
        }
        let sql = if columns.is_empty() {
            format!("INSERT INTO {} DEFAULT VALUES", quote_ident(&def.name))
        } else {
            let placeholders: Vec<String> = (1..=columns.len()).map(|n| format!("?{n}")).collect();
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                quote_ident(&def.name),
                columns.join(", "),
                placeholders.join(", ")
            )
        };
        conn.execute(&sql, rusqlite::params_from_iter(params))
            .map_err(|e| PortError::WriteFailed {
                detail: format!("insert into {:?} failed: {e}", def.name),
            })?;
        Ok(())
    }

    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        let def = self
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?;
        let count = count_rows(conn, &def.name)?;
        let (start, end) = datacraft_core::port::window_slice(table, range, count)?;
        let sql = format!(
            "SELECT * FROM {} LIMIT ?1 OFFSET ?2",
            quote_ident(&def.name)
        );
        let mut statement = conn.prepare(&sql).map_err(|e| read_failed(&def.name, e))?;
        let len = i64::try_from(end - start).unwrap_or(i64::MAX);
        let offset = i64::try_from(start).unwrap_or(i64::MAX);
        let mut rows = statement
            .query(rusqlite::params![len, offset])
            .map_err(|e| read_failed(&def.name, e))?;
        let mut out = Vec::with_capacity(end - start);
        while let Some(row) = rows.next().map_err(|e| read_failed(&def.name, e))? {
            let mut values = Vec::with_capacity(def.fields.len());
            for (index, field) in def.fields.iter().enumerate() {
                let value = row.get_ref(index).map_err(|e| read_failed(&def.name, e))?;
                values.push(to_field_value(&field.kind, value));
            }
            out.push(values);
        }
        Ok(out)
    }

    fn relationships(&self) -> Result<Vec<Relationship>, PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        let mut out = Vec::new();
        for table in &self.tables {
            out.extend(foreign_keys(conn, table)?);
        }
        Ok(out)
    }
}

mod mapping;

use mapping::{
    collect_tables, count_rows, create_sql, foreign_keys, no_database, open_writable, quote_ident,
    read_failed, read_magic, sql_value, to_field_value,
};
