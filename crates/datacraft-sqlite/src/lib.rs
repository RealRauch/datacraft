//! SQLite backend behind the core [`StoragePort`] (steps 13.1/13.2, H35/H36).
//!
//! SQLite is the second source family next to Jet/ACE: it is opened by file
//! magic, catalogued through `sqlite_master`, and mapped onto the same
//! domain vocabulary. The affinity table (SQLite declared type -> Access
//! field kind) is documented in `docs/FORMAT-JET.md` §"SQLite backend".

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, OpenedDatabase, PortError, Relationship,
    RowRange, StoragePort, Support,
};
use datacraft_core::schema::{FieldKind, FieldValue, TableDef};
use rusqlite::Connection;

/// SQLite backend over one open database file.
#[derive(Debug, Default)]
pub struct SqlitePort {
    conn: Option<Connection>,
    tables: Vec<TableDef>,
    path: Option<PathBuf>,
    /// Whether the file opened read-write (read-only files still read).
    writable: bool,
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

impl SqlitePort {
    /// Shared insert path (step 28.1, item U14): `allow_autonumber` is the
    /// restore exception, where the image already carries the identity.
    fn insert_values(
        &mut self,
        table: &str,
        values: &[FieldValue],
        allow_autonumber: bool,
    ) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
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
                    if allow_autonumber {
                        return Err(PortError::WriteFailed {
                            detail: format!(
                                "restoring a row needs the original AutoNumber value of field {:?}",
                                field.name
                            ),
                        });
                    }
                    continue;
                }
                if !allow_autonumber {
                    return Err(PortError::WriteFailed {
                        detail: format!(
                            "autonumber field {:?} is engine-assigned; user-provided values are rejected",
                            field.name
                        ),
                    });
                }
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
}

impl StoragePort for SqlitePort {
    fn driver_id(&self) -> DriverId {
        DriverId::new("sqlite")
    }

    /// Per-session capabilities (phase 30, M17; ADR-7): table renames ride
    /// the native `ALTER TABLE`; field-level alterations, batches one by
    /// one (no transaction wrapper), passwords do not exist, and the
    /// inventories Jet exposes (native queries, macros, VBA) have no
    /// SQLite counterpart. A read-only file disables the write
    /// capabilities; file creation is session-independent.
    fn capabilities(&self) -> DriverCapabilities {
        let mut caps = DriverCapabilities::full();
        caps.write_rows = self.writable;
        caps.create_table = self.writable;
        caps.drop_table = self.writable;
        caps.alter_table = false;
        caps.rename_table = self.writable;
        caps.atomic_batch = false;
        caps.passwords = false;
        caps.relationships = Support::Yes;
        caps.autonumber = Support::Yes; // single AutoNumber via AUTOINCREMENT
        caps.native_queries = Support::No;
        caps.macros = Support::No;
        caps.vba = Support::No;
        caps
    }

    /// SQLite applies sidecar views natively (step 71.3, item M93).
    fn supports_native_views(&self) -> bool {
        true
    }

    fn create_native_view(&mut self, name: &str, sql: &str) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        let statement = format!(
            "CREATE VIEW {} AS {}",
            crate::mapping::quote_ident(name),
            sql.trim_end().trim_end_matches(';')
        );
        conn.execute_batch(&statement)
            .map_err(|error| PortError::WriteFailed {
                detail: error.to_string(),
            })
    }

    fn drop_native_view(&mut self, name: &str) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        let statement = format!("DROP VIEW IF EXISTS {}", crate::mapping::quote_ident(name));
        conn.execute_batch(&statement)
            .map_err(|error| PortError::WriteFailed {
                detail: error.to_string(),
            })
    }

    fn open(&mut self, path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        // SQLite has no database password; the flag is ignored.
        read_magic(path)?;
        let (conn, writable) = open_database(path)?;
        let tables = collect_tables(&conn)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.conn = Some(conn);
        self.tables = tables;
        self.path = Some(path.to_path_buf());
        self.writable = writable;
        Ok(OpenedDatabase {
            name,
            format: DatabaseFormat::Sqlite,
            lock_file: None,
        })
    }

    fn close(&mut self) {
        self.conn = None;
        self.tables.clear();
        self.path = None;
        self.writable = false;
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        if format != DatabaseFormat::Sqlite {
            return Err(PortError::Unsupported {
                reason: "the SQLite backend creates SQLite databases only — use the Jet \
                         backend for Jet4 files"
                    .to_owned(),
            });
        }
        if path.exists() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "{}: file already exists — creating would overwrite it",
                    path.display()
                ),
            });
        }
        // `Connection::open` creates a zero-byte file; VACUUM materializes
        // the real database header so format detection sniffs the fresh
        // file. A failed create leaves no debris behind.
        let outcome = initialize_sqlite_file(path);
        if outcome.is_err() {
            let _ = std::fs::remove_file(path);
        }
        outcome
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        definition.validate().map_err(|e| PortError::WriteFailed {
            detail: e.to_string(),
        })?;
        let create = create_sql(definition)?;
        let create = create.trim_end().trim_end_matches(';');
        let mut sql = format!(
            "BEGIN;
{create};
"
        );
        for field in &definition.fields {
            let suffix = match field.properties.indexed {
                datacraft_core::schema::IndexMode::None => continue,
                datacraft_core::schema::IndexMode::DuplicatesOk => "",
                datacraft_core::schema::IndexMode::NoDuplicates => "UNIQUE ",
            };
            let index_name = format!("idx_{}_{}", definition.name, field.name);
            sql.push_str(&format!(
                "
CREATE {suffix}INDEX {} ON {} ({});",
                quote_ident(&index_name),
                quote_ident(&definition.name),
                quote_ident(&field.name)
            ));
        }
        sql.push_str(
            "
COMMIT;",
        );
        // `execute_batch` stops at the first failing statement without
        // closing the transaction (M25): roll back explicitly so a failed
        // CREATE INDEX cannot leave the connection inside an open
        // transaction with a half-created table.
        if let Err(error) = conn.execute_batch(&sql) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(PortError::WriteFailed {
                detail: format!("creating table {:?} failed: {error}", definition.name),
            });
        }
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
        if !self.writable {
            return Err(read_only());
        }
        conn.execute(&format!("DROP TABLE {}", quote_ident(&canonical)), [])
            .map_err(|e| PortError::WriteFailed {
                detail: format!("dropping table {canonical:?} failed: {e}"),
            })?;
        self.tables.remove(pos);
        Ok(canonical)
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> Result<String, PortError> {
        let Some(pos) = self
            .tables
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return Err(PortError::TableNotFound(table.to_owned()));
        };
        let canonical = self.tables[pos].name.clone();
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        if canonical.eq_ignore_ascii_case(new_name) && canonical != new_name {
            return Err(PortError::Unsupported {
                reason: format!(
                    "SQLite cannot change the case of a table name in place \
                     ({canonical:?} -> {new_name:?}) — SQLite compares names case-insensitively"
                ),
            });
        }
        if self
            .tables
            .iter()
            .enumerate()
            .any(|(index, t)| index != pos && t.name.eq_ignore_ascii_case(new_name))
        {
            return Err(PortError::WriteFailed {
                detail: format!("duplicate table name {new_name:?}"),
            });
        }
        conn.execute(
            &format!(
                "ALTER TABLE {} RENAME TO {}",
                quote_ident(&canonical),
                quote_ident(new_name)
            ),
            [],
        )
        .map_err(|e| PortError::WriteFailed {
            detail: format!("renaming table {canonical:?} failed: {e}"),
        })?;
        self.tables[pos].name = new_name.to_owned();
        Ok(new_name.to_owned())
    }

    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        self.insert_values(table, values, false)
    }

    /// Restores a row image verbatim, including its AutoNumber identity
    /// (step 28.1, item U14).
    fn restore_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        self.insert_values(table, values, true)
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

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        let def = self
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?
            .clone();
        if values.len() != def.fields.len() {
            return Err(PortError::WriteFailed {
                detail: format!(
                    "update {:?}: expected {} values, found {}",
                    def.name,
                    def.fields.len(),
                    values.len()
                ),
            });
        }
        // AutoNumber columns keep their engine-assigned value.
        let mut assignments = Vec::new();
        let mut params = Vec::new();
        for (field, value) in def.fields.iter().zip(values) {
            if matches!(field.kind, FieldKind::AutoNumber(_)) {
                continue;
            }
            params.push(sql_value(&field.kind, value));
            assignments.push(format!("{} = ?{}", quote_ident(&field.name), params.len()));
        }
        if assignments.is_empty() {
            // Only AutoNumber columns: verify the row exists.
            let count = count_rows(conn, &def.name)?;
            return if row < count {
                Ok(())
            } else {
                Err(out_of_range(table, row, count))
            };
        }
        params.push(rusqlite::types::Value::Integer(
            i64::try_from(row).unwrap_or(i64::MAX),
        ));
        let sql = format!(
            "UPDATE {} SET {} WHERE rowid = (SELECT rowid FROM {} ORDER BY rowid LIMIT 1 OFFSET ?{})",
            quote_ident(&def.name),
            assignments.join(", "),
            quote_ident(&def.name),
            params.len()
        );
        let affected = conn
            .execute(&sql, rusqlite::params_from_iter(params))
            .map_err(|e| PortError::WriteFailed {
                detail: format!("update of {:?} failed: {e}", def.name),
            })?;
        if affected == 0 {
            return Err(out_of_range(table, row, count_rows(conn, &def.name)?));
        }
        Ok(())
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        let def = self
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
            .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?;
        let sql = format!(
            "DELETE FROM {} WHERE rowid = (SELECT rowid FROM {} ORDER BY rowid LIMIT 1 OFFSET ?1)",
            quote_ident(&def.name),
            quote_ident(&def.name)
        );
        let offset = i64::try_from(row).unwrap_or(i64::MAX);
        let affected =
            conn.execute(&sql, rusqlite::params![offset])
                .map_err(|e| PortError::WriteFailed {
                    detail: format!("delete from {:?} failed: {e}", def.name),
                })?;
        if affected == 0 {
            return Err(out_of_range(table, row, count_rows(conn, &def.name)?));
        }
        Ok(())
    }

    fn create_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        relationships::create(self, relationship)
    }

    fn delete_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        relationships::delete(self, relationship)
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

mod ddl;
mod mapping;
mod relationships;

use mapping::{
    collect_tables, count_rows, create_sql, foreign_keys, no_database, open_database, quote_ident,
    read_failed, read_magic, read_only, sql_value, to_field_value,
};

/// Out-of-range error for a single addressed row.
fn out_of_range(table: &str, row: usize, row_count: usize) -> PortError {
    PortError::OutOfRange {
        table: table.to_owned(),
        start: row,
        len: 1,
        row_count,
    }
}

/// Creates and initializes a brand-new SQLite database file.
fn initialize_sqlite_file(path: &Path) -> Result<(), PortError> {
    let failed = |stage: &str, e: rusqlite::Error| PortError::WriteFailed {
        detail: format!("{}: {stage}: {e}", path.display()),
    };
    let conn = Connection::open(path).map_err(|e| failed("opening", e))?;
    conn.execute_batch("VACUUM")
        .map_err(|e| failed("initializing", e))?;
    conn.close().map_err(|(_, e)| failed("closing", e))?;
    Ok(())
}
