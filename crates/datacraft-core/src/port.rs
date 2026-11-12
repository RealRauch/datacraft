//! Storage port (step 4.1, item H13).
//!
//! The command core performs storage I/O only through this core-owned
//! trait: real Jet/ACE backends implement it (dependency direction
//! `storage → core`), headless tests use [`MockPort`]. No storage type
//! crosses into the core vocabulary.

use serde::{Deserialize, Serialize};

use crate::schema::{FieldValue, TableDef};

mod capabilities;
mod native_query;
mod native_view;
mod source_objects;
mod write_op;

pub use capabilities::{DriverCapabilities, DriverId, DriverInfo, Support};
pub use native_query::{
    NativeJoin, NativeJoinKind, NativeOrder, NativeQuery, NativeQueryColumn, NativeQueryKind,
};
pub use native_view::NativeView;
pub use source_objects::{
    NativeObject, NativeObjectKind, SourceFact, SourceFacts, SourceIndex, SourceTrigger,
};

/// Binary format family of an opened database (domain vocabulary; the
/// backends map their physical version detection onto it).
///
/// `#[non_exhaustive]` (M32): ADR-7's server path adds format families;
/// clients match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DatabaseFormat {
    /// Jet 3.x (Access 97 and earlier).
    Jet3,
    /// Jet 4.x (Access 2000–2003).
    Jet4,
    /// ACE (Access 2007 and later, `.accdb`).
    Ace,
    /// SQLite 3 (`.sqlite`, `.db`) — the non-Jet source family (H35).
    Sqlite,
    /// A directory of `*.csv` files (phase 30, M18; ADR-7 driver proof).
    Csv,
}

impl DatabaseFormat {
    /// The canonical extension of a family this build can create — without
    /// the dot (step 75.3, item L43). The creation pickers suggest it and
    /// the create commands compare against it.
    pub fn create_extension(self) -> Option<&'static str> {
        match self {
            Self::Jet4 => Some("mdb"),
            Self::Sqlite => Some("sqlite"),
            Self::Jet3 | Self::Ace | Self::Csv => None,
        }
    }

    /// The suffix conflict of a creation path (step 75.3, item L43): the
    /// extension of the *other* creatable family, compared
    /// case-insensitively. `None` accepts matching, unknown and missing
    /// extensions — the create command carries the format explicitly, so
    /// only a known extension of the other family is a contradiction.
    pub fn creation_suffix_conflict(self, path: &std::path::Path) -> Option<&'static str> {
        let extension = path.extension()?.to_str()?;
        [Self::Jet4, Self::Sqlite]
            .into_iter()
            .filter(|family| *family != self)
            .filter_map(Self::create_extension)
            .find(|candidate| extension.eq_ignore_ascii_case(candidate))
    }
}

/// Errors a storage backend reports through the port.
///
/// `#[non_exhaustive]` (M22, ADR-7): adding a variant is not a breaking
/// change for drivers or clients compiled against a published core —
/// out-of-crate consumers match with a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum PortError {
    /// Opening the database file failed.
    OpenFailed {
        /// Concrete, locatable detail.
        detail: String,
    },
    /// The requested table does not exist.
    TableNotFound(String),
    /// A write operation failed.
    WriteFailed {
        /// Concrete, locatable detail.
        detail: String,
    },
    /// The file is valid but uses a feature the backend does not support.
    Unsupported {
        /// What is unsupported and why.
        reason: String,
    },
    /// A row window starts beyond the table's rows.
    OutOfRange {
        /// Name of the table addressed.
        table: String,
        /// First requested row index.
        start: usize,
        /// Requested window length.
        len: usize,
        /// Actual row count of the table.
        row_count: usize,
    },
    /// The database is password-protected and no password was supplied.
    PasswordRequired {
        /// Filesystem path of the database.
        path: String,
    },
    /// The supplied password does not match the stored database password.
    WrongPassword {
        /// Filesystem path of the database.
        path: String,
    },
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed { detail } => write!(f, "database open failed: {detail}"),
            Self::TableNotFound(name) => write!(f, "table {name:?} not found"),
            Self::WriteFailed { detail } => write!(f, "write failed: {detail}"),
            Self::Unsupported { reason } => write!(f, "unsupported: {reason}"),
            Self::OutOfRange {
                table,
                start,
                len,
                row_count,
            } => write!(
                f,
                "row window {start}+{len} of {table:?} is out of range ({row_count} rows)"
            ),
            Self::PasswordRequired { path } => {
                write!(f, "{path}: the database is password-protected")
            }
            Self::WrongPassword { path } => {
                write!(f, "{path}: the database password is wrong")
            }
        }
    }
}

impl std::error::Error for PortError {}

/// One referential-integrity relationship between two tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    /// Table holding the referencing column.
    pub table: String,
    /// Referencing column.
    pub column: String,
    /// Referenced (one-side) table.
    pub related_table: String,
    /// Referenced column.
    pub related_column: String,
}

/// Window selector for row reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowRange {
    /// Zero-based index of the first row to read.
    pub start: usize,
    /// Maximum number of rows to read.
    pub len: usize,
}

/// Metadata of a successfully opened database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenedDatabase {
    /// Database name (file stem).
    pub name: String,
    /// Binary format family.
    pub format: DatabaseFormat,
    /// A sibling Access lock file detected at open (step 53.1, item M59):
    /// the file NAME as data — Access may hold the database open, so the
    /// session opened read-only. Language stays UI-owned (ADR-4); clients
    /// localize the notice from this field. `None` = no lock file seen.
    pub lock_file: Option<String>,
}

/// The storage seam of the command core. Implementors must be deterministic
/// and must not block indefinitely.
pub trait StoragePort {
    /// Stable identity of this driver (phase 30, item M17; ADR-7).
    /// Required (M22): a driver names itself — there is no default
    /// identity, so a driver that forgets its declaration does not
    /// compile instead of hiding behind `"generic"`.
    fn driver_id(&self) -> DriverId;
    /// Declarative capabilities of the open data source (phase 30, item
    /// M17; ADR-7) — per opened session, because they depend on runtime
    /// state. Required (M22): there is no default — the former
    /// optimistic `DriverCapabilities::full()` fallback made a forgetful
    /// driver *lie* about what it can do; now the compiler asks for the
    /// declaration. Start from [`DriverCapabilities::none`] or
    /// [`DriverCapabilities::full`] and state the deviations.
    fn capabilities(&self) -> DriverCapabilities;
    /// The source family this driver opens, when fixed for the driver
    /// (M23). The default is `None` — a single-purpose driver names its
    /// family; a multi-family driver (the Jet/ACE adapter) may too, with
    /// its primary family. The detecting composite reads this for its
    /// inventory's fallback entries.
    fn source_format(&self) -> Option<DatabaseFormat> {
        None
    }
    /// The driver inventory of this composition (M23): every driver it
    /// can open with id, fixed family and offline capability summary.
    /// The default is empty — a single-purpose port names itself via
    /// [`StoragePort::driver_id`]; the detecting composite reports its
    /// full registry so agents can pre-flight an open.
    fn driver_inventory(&self) -> Vec<DriverInfo> {
        Vec::new()
    }
    /// Opens the database at `path`; `password` is `None` for unprotected
    /// files (step 16.2, M5). Backends without passwords ignore it.
    fn open(
        &mut self,
        path: &std::path::Path,
        password: Option<&str>,
    ) -> Result<OpenedDatabase, PortError>;
    /// Closes the open database (idempotent).
    fn close(&mut self);
    /// Creates a new, empty database file at `path` (phase 29, item H44).
    /// The file must not exist yet; creating never opens the database —
    /// the caller dispatches [`StoragePort::open`] afterwards. Backends
    /// that cannot create files answer a precise `Unsupported` (the
    /// default).
    fn create_database(
        &mut self,
        path: &std::path::Path,
        format: DatabaseFormat,
    ) -> Result<(), PortError> {
        let _ = (path, format);
        Err(PortError::Unsupported {
            reason: "creating database files is not supported by this backend".to_owned(),
        })
    }
    /// Writes a copy of the open database to `path` (step 27.2, item
    /// M14). `password` protects the copy; `None` writes an unencrypted
    /// copy. The original stays untouched. Backends without a password
    /// surface answer a precise `Unsupported` (the default).
    fn save_as(&mut self, path: &std::path::Path, password: Option<&str>) -> Result<(), PortError> {
        let _ = (path, password);
        Err(PortError::Unsupported {
            reason: "saving database copies is not supported by this backend".to_owned(),
        })
    }
    /// Replaces the open database's password in place (step 27.2, item
    /// M14); `None` removes the protection. Backends without a password
    /// surface answer a precise `Unsupported` (the default).
    fn change_password(&mut self, new_password: Option<&str>) -> Result<(), PortError> {
        let _ = new_password;
        Err(PortError::Unsupported {
            reason: "changing database passwords is not supported by this backend".to_owned(),
        })
    }
    /// Catalog of table definitions in storage order.
    fn tables(&self) -> Vec<TableDef>;
    /// Views that live in the opened file itself (phase 73, step 73.1;
    /// item M90) — read-only, never DataCraft's design layer. The default
    /// is empty: Jet/ACE and CSV carry none.
    fn native_views(&self) -> Result<Vec<NativeView>, PortError> {
        Ok(Vec::new())
    }
    /// Indexes that live in the opened file itself (phase 73, step 73.2;
    /// item M90) — read-only inventory. The default is empty.
    fn indexes(&self) -> Result<Vec<SourceIndex>, PortError> {
        Ok(Vec::new())
    }
    /// Triggers that live in the opened file itself (phase 73, step 73.2;
    /// item M90) — inventory only, never created or executed. The default
    /// is empty.
    fn triggers(&self) -> Result<Vec<SourceTrigger>, PortError> {
        Ok(Vec::new())
    }
    /// Driver facts and the file's own health verdict (phase 73, step
    /// 73.3; item M91). Read at open: the check is bounded and runs once.
    /// The default reports no facts and a healthy file.
    fn source_facts(&self) -> SourceFacts {
        SourceFacts::default()
    }
    /// Native Access forms/reports named by the catalog (phase 74, step
    /// 74.1; item M87) — read-only inventory, definitions never decoded.
    /// The default is empty.
    fn native_objects(&self) -> Result<Vec<NativeObject>, PortError> {
        Ok(Vec::new())
    }
    /// Creates a table; errors on duplicate names.
    fn create_table(&mut self, definition: &TableDef) -> Result<(), PortError>;
    /// Drops a table; returns its canonical name.
    fn drop_table(&mut self, name: &str) -> Result<String, PortError>;
    /// Reads a window of rows as domain values. A window starting at
    /// `row_count` is empty; anything beyond fails with
    /// [`PortError::OutOfRange`].
    fn read_rows(&self, table: &str, range: RowRange) -> Result<Vec<Vec<FieldValue>>, PortError>;
    /// Appends one row (positional values, arity must match the table).
    /// `Null` on an AutoNumber field means "engine-assigned".
    fn insert_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError>;
    /// Appends a row image verbatim, including engine-assigned AutoNumber
    /// values (step 28.1, item U14 — undo/redo restoration). The default
    /// delegates to [`StoragePort::insert_row`]; backends that reject
    /// user-supplied AutoNumbers override it with a validated restore path.
    fn restore_row(&mut self, table: &str, values: &[FieldValue]) -> Result<(), PortError> {
        self.insert_row(table, values)
    }
    /// Replaces the row at `row` (zero-based live-row index) with `values`
    /// (step 11.10, H40). AutoNumber values are preserved by the caller.
    fn update_row(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError>;
    /// Deletes the row at `row` (zero-based live-row index).
    fn delete_row(&mut self, table: &str, row: usize) -> Result<(), PortError>;
    /// Applies a batch of row operations in order (step 11.11, M2). Indices
    /// address the row set as it evolves through the batch. Backends apply
    /// the batch atomically where they can (Jet does); the default applies
    /// the operations one by one.
    fn write_row_ops(&mut self, table: &str, ops: &[RowWriteOp]) -> Result<(), PortError> {
        for op in ops {
            match op {
                RowWriteOp::Insert(values) => self.insert_row(table, values)?,
                RowWriteOp::Update { row, values } => self.update_row(table, *row, values)?,
                RowWriteOp::Delete { row } => self.delete_row(table, *row)?,
            }
        }
        Ok(())
    }
    /// Alters one table's schema (step 24.3, item M9). Backends that cannot
    /// rewrite their catalog answer a precise `Unsupported`.
    fn alter_table(&mut self, table: &str, change: SchemaChange) -> Result<(), PortError> {
        let _ = (table, change);
        Err(PortError::Unsupported {
            reason: "schema alteration is not supported by this backend".to_owned(),
        })
    }
    /// Renames one table in place (phase 72, item M81); rows, indexes and
    /// relationships follow the name. Gated on `alter_table` by the
    /// dispatcher; backends that cannot rewrite their catalog answer a
    /// precise `Unsupported` (the default).
    fn rename_table(&mut self, table: &str, new_name: &str) -> Result<String, PortError> {
        let _ = (table, new_name);
        Err(PortError::Unsupported {
            reason: "table renames are not supported by this backend".to_owned(),
        })
    }
    /// Whether the backend can apply sidecar views natively (step 71.3,
    /// item M93): SQLite compiles them to `CREATE VIEW`, while Jet/ACE/CSV
    /// rely on the shadow SQL engine and answer `false`.
    fn supports_native_views(&self) -> bool {
        false
    }
    /// Creates one native view in the open database (step 71.3, M93).
    /// Backends without a view engine answer a precise `Unsupported`.
    fn create_native_view(&mut self, name: &str, sql: &str) -> Result<(), PortError> {
        let _ = (name, sql);
        Err(PortError::Unsupported {
            reason: "native views are not supported by this backend".to_owned(),
        })
    }
    /// Drops one native view (step 71.3, M93).
    fn drop_native_view(&mut self, name: &str) -> Result<(), PortError> {
        let _ = name;
        Err(PortError::Unsupported {
            reason: "native views are not supported by this backend".to_owned(),
        })
    }
    /// Referential relationships of the open database.
    fn relationships(&self) -> Result<Vec<Relationship>, PortError>;
    /// Creates one referential relationship (step 32.3, item M16).
    /// Backends without a relationship write path answer a precise
    /// `Unsupported` (the default).
    fn create_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        let _ = relationship;
        Err(PortError::Unsupported {
            reason: "creating relationships is not supported by this backend".to_owned(),
        })
    }
    /// Deletes one referential relationship (step 32.3, item M16).
    fn delete_relationship(&mut self, relationship: &Relationship) -> Result<(), PortError> {
        let _ = relationship;
        Err(PortError::Unsupported {
            reason: "deleting relationships is not supported by this backend".to_owned(),
        })
    }
    /// Native saved queries of the open database (step 15.3, M3).
    /// Backends without native queries return an empty list.
    fn native_queries(&self) -> Result<Vec<NativeQuery>, PortError> {
        Ok(Vec::new())
    }
    /// Named macros of the open database (step 17.1, L4). Backends without
    /// macros return an empty list; definitions are read-only.
    fn macros(&self) -> Result<Vec<crate::macro_def::MacroDef>, PortError> {
        Ok(Vec::new())
    }
    /// Extracted VBA project of the open database (step 17.2, L5).
    /// Backends without a decoded project return `None`; extraction errors
    /// are reported so the core can surface them without blocking the open.
    fn vba_project(&self) -> Result<Option<crate::vba::VbaProject>, PortError> {
        Ok(None)
    }
}

/// One schema change of an `alter_table` call (step 24.3, item M9).
///
/// `#[non_exhaustive]` (M32): new alter operations are additive for
/// out-of-crate drivers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SchemaChange {
    /// Appends one field (the new last column).
    AddField {
        /// Full definition of the new field.
        field: crate::schema::FieldDef,
    },
    /// Removes one field (name matched case-insensitively).
    DropField {
        /// Field name.
        field: String,
    },
    /// Renames one field; row data is untouched.
    RenameField {
        /// Current field name (case-insensitive).
        field: String,
        /// New name.
        new_name: String,
    },
    /// Replaces one field's properties (storage shape must not change).
    SetFieldProperties {
        /// Field name (case-insensitive).
        field: String,
        /// The new property set.
        properties: crate::schema::FieldProperties,
    },
}

pub use write_op::{RowWriteOp, window_slice};

pub use crate::port_mock::MockPort;

#[cfg(test)]
mod tests;
