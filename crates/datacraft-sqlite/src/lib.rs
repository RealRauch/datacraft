//! SQLite backend behind the core [`StoragePort`] (steps 13.1/13.2, H35/H36).
//!
//! SQLite is the second source family next to Jet/ACE: it is opened by file
//! magic, catalogued through `sqlite_master`, and mapped onto the same
//! domain vocabulary. The affinity table (SQLite declared type -> Access
//! field kind) is documented in `docs/FORMAT-JET.md` §"SQLite backend".

use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, DriverCapabilities, DriverId, NativeView, OpenedDatabase, PortError,
    Relationship, RowRange, StoragePort, Support,
};
use datacraft_core::schema::{FieldValue, TableDef};
use rusqlite::Connection;

/// SQLite backend over one open database file.
#[derive(Debug, Default)]
pub struct SqlitePort {
    conn: Option<Connection>,
    tables: Vec<TableDef>,
    /// File-owned views (phase 73, step 73.1; M90) — read-only.
    native_views: Vec<NativeView>,
    /// Read-only index/trigger inventory (phase 73, step 73.2; M90).
    indexes: Vec<datacraft_core::port::SourceIndex>,
    triggers: Vec<datacraft_core::port::SourceTrigger>,
    /// Driver facts and the health verdict (phase 73, step 73.3; M91).
    source_facts: datacraft_core::port::SourceFacts,
    /// Tables declared `WITHOUT ROWID` (phase 73, step 73.4; M92): read
    /// windows follow the primary key, rowid writes answer precisely.
    without_rowid: Vec<String>,
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

    /// Resolves a readable table or file-owned view (M90).
    fn readable(&self, name: &str) -> Result<TableDef, PortError> {
        if let Some(table) = self
            .tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(name))
        {
            return Ok(table.clone());
        }
        if let Some(view) = self
            .native_views
            .iter()
            .find(|view| view.name.eq_ignore_ascii_case(name))
        {
            return Ok(TableDef::new(&view.name, view.fields.clone()));
        }
        Err(PortError::TableNotFound(name.to_owned()))
    }

    /// Precise rowid-addressing rejection for `WITHOUT ROWID` tables
    /// (phase 73, step 73.4; M92).
    fn reject_without_rowid(&self, name: &str) -> Result<(), PortError> {
        if self
            .without_rowid
            .iter()
            .any(|table| table.eq_ignore_ascii_case(name))
        {
            return Err(PortError::Unsupported {
                reason: format!(
                    "{name:?} is a WITHOUT ROWID table — row addressing by rowid is not \
                     supported; edit the rows in SQLite directly"
                ),
            });
        }
        Ok(())
    }

    /// Precise write rejection for a file-owned view (M90).
    fn reject_view(&self, name: &str) -> Result<(), PortError> {
        if self
            .native_views
            .iter()
            .any(|view| view.name.eq_ignore_ascii_case(name))
        {
            return Err(view_inventory::read_only(name));
        }
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
        // Triggers are exposed read-only (inventory, no execute/create —
        // phase 73, step 73.2; M90).
        caps.triggers = Support::ReadOnly;
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
            })?;
        self.native_views = view_inventory::collect(conn)?;
        Ok(())
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
            })?;
        self.native_views = view_inventory::collect(conn)?;
        Ok(())
    }

    fn open(&mut self, path: &Path, _password: Option<&str>) -> Result<OpenedDatabase, PortError> {
        // SQLite has no database password; the flag is ignored.
        read_magic(path)?;
        let (conn, writable) = open_database(path)?;
        let tables = collect_tables(&conn)?;
        let native_views = view_inventory::collect(&conn)?;
        let indexes = source_objects::indexes(&conn)?;
        let triggers = source_objects::triggers(&conn)?;
        let source_facts = source_objects::facts(&conn);
        let (_, without_rowid) = source_objects::table_options(&conn)?;
        let name = path.file_stem().map_or_else(
            || "database".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        self.conn = Some(conn);
        self.tables = tables;
        self.native_views = native_views;
        self.indexes = indexes;
        self.triggers = triggers;
        self.source_facts = source_facts;
        self.without_rowid = without_rowid;
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
        self.native_views.clear();
        self.indexes.clear();
        self.triggers.clear();
        self.source_facts = datacraft_core::port::SourceFacts::default();
        self.without_rowid.clear();
        self.path = None;
        self.writable = false;
    }

    fn create_database(&mut self, path: &Path, format: DatabaseFormat) -> Result<(), PortError> {
        lifecycle::create(path, format)
    }

    fn tables(&self) -> Vec<TableDef> {
        self.tables.clone()
    }

    fn native_views(&self) -> Result<Vec<NativeView>, PortError> {
        Ok(self.native_views.clone())
    }

    fn indexes(&self) -> Result<Vec<datacraft_core::port::SourceIndex>, PortError> {
        Ok(self.indexes.clone())
    }

    fn triggers(&self) -> Result<Vec<datacraft_core::port::SourceTrigger>, PortError> {
        Ok(self.triggers.clone())
    }

    fn source_facts(&self) -> datacraft_core::port::SourceFacts {
        self.source_facts.clone()
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
        self.read_window(table, range)
    }

    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        self.write_update(table, row, values)
    }

    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        self.write_delete(table, row)
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
mod lifecycle;
mod mapping;
mod relationships;
mod rows;
mod rows_io;
mod source_objects;
mod values;
mod view_inventory;

use mapping::{
    collect_tables, create_sql, foreign_keys, no_database, open_database, quote_ident, read_magic,
    read_only,
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
