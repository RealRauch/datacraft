//! SQLite catalog/schema/value mapping helpers (steps 13.1/13.2).
//!
//! Kept separate from the port implementation so every file stays under the
//! size gate; the affinity table is documented in `docs/FORMAT-JET.md`.

use std::io::Read;
use std::path::Path;

use crate::ddl::pk_is_autoincrement;
use datacraft_core::port::{PortError, Relationship};
use datacraft_core::schema::{AutoKind, BinaryKind, FieldDef, FieldKind, NumericKind, TableDef};
use rusqlite::{Connection, OpenFlags};

// Value conversion lives in `values.rs`; the re-export keeps the call
// sites (`rows.rs`, `lib.rs`) stable.
pub(crate) use crate::values::{sql_value, to_field_value};

/// The 16-byte file magic of every SQLite 3 database.
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\x00";

/// `CREATE TABLE` statement for a domain definition.
///
/// Lossiness (documented in docs/FORMAT-JET.md): SQLite has no storage
/// classes for byte/integer widths, decimals, dates or booleans, so the
/// declared type is a marker that this backend reads back faithfully;
/// `AUTOINCREMENT` requires an `INTEGER PRIMARY KEY` and at most one such
/// column exists per table.
pub(crate) fn create_sql(definition: &TableDef) -> Result<String, PortError> {
    create_sql_named(definition, &definition.name, &[])
}

/// `CREATE TABLE` for `name`, optionally carrying foreign-key clauses
/// (step 32.3, item M16 rebuild path).
pub(crate) fn create_sql_named(
    definition: &TableDef,
    name: &str,
    foreign_keys: &[Relationship],
) -> Result<String, PortError> {
    let mut columns = Vec::with_capacity(definition.fields.len() + foreign_keys.len());
    let mut auto_seen = false;
    for field in &definition.fields {
        let mut column = format!("{} {}", quote_ident(&field.name), sql_type(&field.kind)?);
        if matches!(field.kind, FieldKind::AutoNumber(_)) {
            if auto_seen {
                return Err(PortError::Unsupported {
                    reason: format!(
                        "table {:?} declares more than one AutoNumber — SQLite allows one \
                         INTEGER PRIMARY KEY AUTOINCREMENT per table",
                        definition.name
                    ),
                });
            }
            auto_seen = true;
            column.push_str(" PRIMARY KEY AUTOINCREMENT");
        } else if field.properties.required {
            column.push_str(" NOT NULL");
        }
        columns.push(column);
    }
    for fk in foreign_keys {
        columns.push(format!(
            "FOREIGN KEY ({}) REFERENCES {} ({})",
            quote_ident(&fk.column),
            quote_ident(&fk.related_table),
            quote_ident(&fk.related_column)
        ));
    }
    Ok(format!(
        "CREATE TABLE {} ({})",
        quote_ident(name),
        columns.join(", ")
    ))
}

/// Domain kind -> SQLite declared type (the inverse of `field_kind`).
pub(crate) fn sql_type(kind: &FieldKind) -> Result<String, PortError> {
    Ok(match kind {
        FieldKind::Text { max_len } => format!("VARCHAR({max_len})"),
        FieldKind::Memo => "TEXT".to_owned(),
        FieldKind::Number(NumericKind::Byte | NumericKind::Integer | NumericKind::LongInteger) => {
            "INTEGER".to_owned()
        }
        FieldKind::Number(NumericKind::Single | NumericKind::Double) => "REAL".to_owned(),
        FieldKind::Number(NumericKind::Decimal { .. }) => "NUMERIC".to_owned(),
        FieldKind::DateTime => "DATETIME".to_owned(),
        FieldKind::Currency => "CURRENCY".to_owned(),
        FieldKind::AutoNumber(AutoKind::LongInteger) => "INTEGER".to_owned(),
        FieldKind::AutoNumber(AutoKind::ReplicationId) => {
            return Err(PortError::Unsupported {
                reason: "SQLite has no replication-id AutoNumber; use a plain GUID field"
                    .to_owned(),
            });
        }
        FieldKind::Boolean { .. } => "BOOLEAN".to_owned(),
        FieldKind::Binary(_) => "BLOB".to_owned(),
        FieldKind::MultiValue(_) | FieldKind::Complex => {
            return Err(PortError::Unsupported {
                reason: "SQLite has no ACE complex values (attachments, multi-value)".to_owned(),
            });
        }
        FieldKind::Guid => "GUID".to_owned(),
    })
}

/// Reads the file's leading 16 bytes and rejects non-SQLite content.
pub(crate) fn read_magic(path: &Path) -> Result<(), PortError> {
    let mut file = std::fs::File::open(path).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", path.display()),
    })?;
    let mut magic = [0u8; 16];
    let mut filled = 0;
    while filled < magic.len() {
        match file.read(&mut magic[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) => {
                return Err(PortError::OpenFailed {
                    detail: format!("{}: {e}", path.display()),
                });
            }
        }
    }
    if filled < magic.len() || &magic != SQLITE_MAGIC {
        return Err(PortError::OpenFailed {
            detail: format!(
                "{}: not a SQLite database (missing \"SQLite format 3\" magic)",
                path.display()
            ),
        });
    }
    Ok(())
}

/// Opens read-write so writes work; a read-only file still opens for
/// reading and reports a precise `Unsupported` on write attempts.
pub(crate) fn open_database(path: &Path) -> Result<(Connection, bool), PortError> {
    // Foreign-key constraints are declared in the DDL (step 32.3, M16);
    // SQLite only enforces them when the pragma is on for the connection.
    let enable = |conn: Connection| -> Result<Connection, PortError> {
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| PortError::OpenFailed {
                detail: format!("{}: enabling foreign keys: {e}", path.display()),
            })?;
        // Bounded writer contention (phase 73, step 73.4; item M92): a
        // short lock held by another writer is waited out instead of
        // answering a bare SQLITE_BUSY. The bound is explicit and visible
        // as the `busy_timeout` fact.
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| PortError::OpenFailed {
                detail: format!("{}: setting the busy timeout: {e}", path.display()),
            })?;
        Ok(conn)
    };
    // Windows lets a read-write open succeed on a read-only file and fails
    // only on the first write; the metadata check keeps the answer precise.
    let readonly_file = std::fs::metadata(path)
        .map(|meta| meta.permissions().readonly())
        .unwrap_or(false);
    if readonly_file {
        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| {
                PortError::OpenFailed {
                    detail: format!("{}: {e}", path.display()),
                }
            })?;
        return enable(conn).map(|conn| (conn, false));
    }
    let rw = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE);
    match rw {
        Ok(conn) => enable(conn).map(|conn| (conn, true)),
        Err(rw_error) => {
            let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|_| PortError::OpenFailed {
                    detail: format!("{}: {rw_error}", path.display()),
                })?;
            enable(conn).map(|conn| (conn, false))
        }
    }
}

/// Precise write rejection for read-only files.
pub(crate) fn read_only() -> PortError {
    PortError::Unsupported {
        reason: "the SQLite database file is read-only".to_owned(),
    }
}

/// Catalog of user tables in creation order (internal `sqlite_*` tables
/// stay hidden — they are SQLite's implementation detail).
pub(crate) fn collect_tables(conn: &Connection) -> Result<Vec<TableDef>, PortError> {
    let mut statement = conn
        .prepare("SELECT name, sql FROM sqlite_master WHERE type = 'table' ORDER BY rowid")
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut rows = statement
        .query([])
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut tables = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed("sqlite_master", e))? {
        let name: String = row.get(0).map_err(|e| read_failed("sqlite_master", e))?;
        if name.starts_with("sqlite_") {
            continue;
        }
        let sql: Option<String> = row.get(1).map_err(|e| read_failed("sqlite_master", e))?;
        // The AutoNumber flag comes from the key column's own DDL
        // segment only — a word mention elsewhere never flips the
        // integer key (step 42.1, L13).
        let key_column = primary_key(conn, &name)?;
        let autoincrement = sql
            .as_deref()
            .zip(key_column.as_deref())
            .is_some_and(|(sql, column)| pk_is_autoincrement(sql, column));
        tables.push(TableDef::new(
            name.clone(),
            table_info(conn, &name, autoincrement)?,
        ));
    }
    Ok(tables)
}

/// `PRAGMA table_info` as ordered field definitions.
pub(crate) fn table_info(
    conn: &Connection,
    table: &str,
    autoincrement: bool,
) -> Result<Vec<FieldDef>, PortError> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({})", quote_ident(table)))
        .map_err(|e| read_failed(table, e))?;
    let mut rows = statement.query([]).map_err(|e| read_failed(table, e))?;
    let mut fields = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed(table, e))? {
        let name: String = row.get(1).map_err(|e| read_failed(table, e))?;
        let declared: String = row.get(2).map_err(|e| read_failed(table, e))?;
        let not_null: i64 = row.get(3).map_err(|e| read_failed(table, e))?;
        let pk: i64 = row.get(5).map_err(|e| read_failed(table, e))?;
        let nullable = not_null == 0;
        let kind = field_kind(&declared, pk, autoincrement, nullable);
        let mut field = FieldDef::new(name, kind);
        field.properties.required = !nullable || matches!(kind, FieldKind::AutoNumber(_));
        fields.push(field);
    }
    Ok(fields)
}

/// SQLite declared type -> Access field kind (affinity table).
///
/// Declared types are matched by name first (BOOLEAN/DATE/CURRENCY/GUID have
/// no SQLite semantics of their own but are stable marker types), then by
/// SQLite's own affinity rules. Marker names may carry a length or
/// precision suffix (`DATETIME(6)` matches the `DATETIME` marker) while
/// the affinity fallbacks and the text budget still see the full
/// declared type. Unbounded text maps to the Access maximum (255);
/// `VARCHAR(n)`/`CHAR(n)` keep their declared budget.
pub(crate) fn field_kind(
    declared: &str,
    pk: i64,
    autoincrement: bool,
    nullable: bool,
) -> FieldKind {
    let upper = declared.trim().to_ascii_uppercase();
    let marker = marker_base(&upper);
    if autoincrement && pk > 0 && marker == "INTEGER" {
        return FieldKind::AutoNumber(AutoKind::LongInteger);
    }
    if marker == "BOOLEAN" || marker == "BOOL" {
        return FieldKind::Boolean {
            tri_state: nullable,
        };
    }
    if matches!(marker, "DATE" | "DATETIME" | "TIMESTAMP") {
        return FieldKind::DateTime;
    }
    if matches!(marker, "CURRENCY" | "MONEY") {
        return FieldKind::Currency;
    }
    if matches!(marker, "GUID" | "UNIQUEIDENTIFIER") {
        return FieldKind::Guid;
    }
    if upper.contains("INT") {
        return FieldKind::Number(NumericKind::LongInteger);
    }
    if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
        return FieldKind::Text {
            max_len: text_budget(&upper),
        };
    }
    if upper.contains("BLOB") || upper.is_empty() {
        return FieldKind::Binary(BinaryKind::OleObject);
    }
    if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
        return FieldKind::Number(NumericKind::Double);
    }
    FieldKind::Number(NumericKind::Decimal {
        precision: 28,
        scale: 0,
    })
}

/// Marker name with a trailing parenthesized suffix removed
/// (`DATETIME(6)` matches the `DATETIME` marker).
fn marker_base(upper: &str) -> &str {
    if upper.ends_with(')')
        && let Some(open) = upper.rfind('(')
    {
        return upper[..open].trim_end();
    }
    upper
}

/// Character budget of a text column: `VARCHAR(n)`/`CHAR(n)` if declared,
/// otherwise the Access maximum of 255.
pub(crate) fn text_budget(declared: &str) -> u16 {
    let Some(open) = declared.find('(') else {
        return 255;
    };
    let Some(close) = declared[open..].find(')') else {
        return 255;
    };
    declared[open + 1..open + close]
        .trim()
        .parse::<u16>()
        .map_or(255, |n| n.clamp(1, 255))
}

/// Foreign keys of one table as port relationships.
pub(crate) fn foreign_keys(
    conn: &Connection,
    table: &TableDef,
) -> Result<Vec<Relationship>, PortError> {
    let mut statement = conn
        .prepare(&format!(
            "PRAGMA foreign_key_list({})",
            quote_ident(&table.name)
        ))
        .map_err(|e| read_failed(&table.name, e))?;
    let mut rows = statement
        .query([])
        .map_err(|e| read_failed(&table.name, e))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed(&table.name, e))? {
        let related_table: String = row.get(2).map_err(|e| read_failed(&table.name, e))?;
        let column: String = row.get(3).map_err(|e| read_failed(&table.name, e))?;
        let target: Option<String> = row.get(4).map_err(|e| read_failed(&table.name, e))?;
        let related_column = match target {
            Some(name) => name,
            None => primary_key(conn, &related_table)?.ok_or_else(|| PortError::Unsupported {
                reason: format!(
                    "foreign key {}.{} references {related_table} without a resolvable primary key",
                    table.name, column
                ),
            })?,
        };
        out.push(Relationship {
            table: table.name.clone(),
            column,
            related_table,
            related_column,
        });
    }
    Ok(out)
}

/// Primary-key columns of `table` in key order (composite keys included;
/// `WITHOUT ROWID` windows order by exactly this, item M92).
pub(crate) fn pk_columns(conn: &Connection, table: &str) -> Result<Vec<String>, PortError> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({})", quote_ident(table)))
        .map_err(|e| read_failed(table, e))?;
    let mut rows = statement.query([]).map_err(|e| read_failed(table, e))?;
    let mut keyed: Vec<(i64, String)> = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed(table, e))? {
        let name: String = row.get(1).map_err(|e| read_failed(table, e))?;
        let pk: i64 = row.get(5).map_err(|e| read_failed(table, e))?;
        if pk > 0 {
            keyed.push((pk, name));
        }
    }
    keyed.sort_by_key(|(order, _)| *order);
    Ok(keyed.into_iter().map(|(_, name)| name).collect())
}

/// Name of the single-column primary key of `table`, if any.
pub(crate) fn primary_key(conn: &Connection, table: &str) -> Result<Option<String>, PortError> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({})", quote_ident(table)))
        .map_err(|e| read_failed(table, e))?;
    let mut rows = statement.query([]).map_err(|e| read_failed(table, e))?;
    while let Some(row) = rows.next().map_err(|e| read_failed(table, e))? {
        let pk: i64 = row.get(5).map_err(|e| read_failed(table, e))?;
        if pk == 1 {
            let name: String = row.get(1).map_err(|e| read_failed(table, e))?;
            return Ok(Some(name));
        }
    }
    Ok(None)
}

pub(crate) fn count_rows(conn: &Connection, table: &str) -> Result<usize, PortError> {
    let sql = format!("SELECT COUNT(*) FROM {}", quote_ident(table));
    let count: i64 = conn
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|e| read_failed(table, e))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

/// Quotes a catalog-provided identifier for interpolation (LESSONS #5:
/// identifiers are whitelisted by the catalog, never user input).
pub(crate) fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub(crate) fn read_failed(table: &str, error: rusqlite::Error) -> PortError {
    PortError::OpenFailed {
        detail: format!("reading {table:?} failed: {error}"),
    }
}

pub(crate) fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
