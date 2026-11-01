//! SQLite backend behind the core [`StoragePort`] (step 13.1, item H35).
//!
//! SQLite is the second source family next to Jet/ACE: it is opened by file
//! magic, catalogued through `sqlite_master`, and mapped onto the same
//! domain vocabulary. The affinity table (SQLite declared type -> Access
//! field kind) is documented in `docs/FORMAT-JET.md` §"SQLite backend".

use std::io::Read;
use std::path::{Path, PathBuf};

use datacraft_core::port::{
    DatabaseFormat, OpenedDatabase, PortError, Relationship, RowRange, StoragePort,
};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

/// The 16-byte file magic of every SQLite 3 database.
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

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

    fn create_table(&mut self, _definition: &TableDef) -> Result<(), PortError> {
        Err(write_pending())
    }

    fn drop_table(&mut self, _name: &str) -> Result<String, PortError> {
        Err(write_pending())
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

/// Reads the file's leading 16 bytes and rejects non-SQLite content.
fn read_magic(path: &Path) -> Result<(), PortError> {
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

/// Opens read-write so later write steps work; a read-only file still
/// opens (the write path then reports the backend error).
fn open_writable(path: &Path) -> Result<Connection, PortError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE;
    Connection::open_with_flags(path, flags).map_err(|e| PortError::OpenFailed {
        detail: format!("{}: {e}", path.display()),
    })
}

/// Catalog of user tables in creation order (internal `sqlite_*` tables
/// stay hidden — they are SQLite's implementation detail).
fn collect_tables(conn: &Connection) -> Result<Vec<TableDef>, PortError> {
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
        let autoincrement = sql.is_some_and(|s| s.to_ascii_uppercase().contains("AUTOINCREMENT"));
        tables.push(TableDef::new(
            name.clone(),
            table_info(conn, &name, autoincrement)?,
        ));
    }
    Ok(tables)
}

/// `PRAGMA table_info` as ordered field definitions.
fn table_info(
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
/// SQLite's own affinity rules. Unbounded text maps to the Access maximum
/// (255); `VARCHAR(n)`/`CHAR(n)` keep their declared budget.
fn field_kind(declared: &str, pk: i64, autoincrement: bool, nullable: bool) -> FieldKind {
    let upper = declared.trim().to_ascii_uppercase();
    if autoincrement && pk > 0 && upper == "INTEGER" {
        return FieldKind::AutoNumber(AutoKind::LongInteger);
    }
    if upper == "BOOLEAN" || upper == "BOOL" {
        return FieldKind::Boolean {
            tri_state: nullable,
        };
    }
    if matches!(upper.as_str(), "DATE" | "DATETIME" | "TIMESTAMP") {
        return FieldKind::DateTime;
    }
    if matches!(upper.as_str(), "CURRENCY" | "MONEY") {
        return FieldKind::Currency;
    }
    if matches!(upper.as_str(), "GUID" | "UNIQUEIDENTIFIER") {
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

/// Character budget of a text column: `VARCHAR(n)`/`CHAR(n)` if declared,
/// otherwise the Access maximum of 255.
fn text_budget(declared: &str) -> u16 {
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

/// Converts one stored cell using the column's domain kind (booleans,
/// currency, decimal scale and GUID blobs need the schema context).
fn to_field_value(kind: &FieldKind, value: ValueRef<'_>) -> FieldValue {
    match value {
        ValueRef::Null => FieldValue::Null,
        ValueRef::Integer(i) => match kind {
            FieldKind::Boolean { .. } => FieldValue::Boolean(i != 0),
            FieldKind::Currency => FieldValue::Currency(i),
            FieldKind::DateTime => FieldValue::DateTime(i as f64),
            FieldKind::Number(NumericKind::Decimal { scale, .. }) => {
                FieldValue::Decimal(i128::from(i) * decimal_factor(u32::from(*scale)))
            }
            _ => FieldValue::Integer(i),
        },
        ValueRef::Real(f) => match kind {
            FieldKind::DateTime => FieldValue::DateTime(f),
            FieldKind::Number(NumericKind::Decimal { scale, .. }) => {
                FieldValue::Decimal((f * decimal_factor(u32::from(*scale)) as f64).round() as i128)
            }
            _ => FieldValue::Float(f),
        },
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).into_owned();
            match kind {
                FieldKind::Guid => {
                    parse_guid(&text).map_or(FieldValue::Text(text), FieldValue::Guid)
                }
                _ => FieldValue::Text(text),
            }
        }
        ValueRef::Blob(bytes) => match kind {
            FieldKind::Guid => <[u8; 16]>::try_from(bytes)
                .map_or_else(|_| FieldValue::Binary(bytes.to_vec()), FieldValue::Guid),
            _ => FieldValue::Binary(bytes.to_vec()),
        },
    }
}

/// 10^scale as an exact integer factor for decimal scaling.
fn decimal_factor(scale: u32) -> i128 {
    10i128.pow(scale.min(28))
}

/// Canonical 8-4-4-4-12 hex GUID text (with or without braces).
fn parse_guid(text: &str) -> Option<[u8; 16]> {
    let hex: String = text
        .trim_matches(|c| c == '{' || c == '}')
        .chars()
        .filter(|c| *c != '-')
        .collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Foreign keys of one table as port relationships.
fn foreign_keys(conn: &Connection, table: &TableDef) -> Result<Vec<Relationship>, PortError> {
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

/// Name of the single-column primary key of `table`, if any.
fn primary_key(conn: &Connection, table: &str) -> Result<Option<String>, PortError> {
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

fn count_rows(conn: &Connection, table: &str) -> Result<usize, PortError> {
    let sql = format!("SELECT COUNT(*) FROM {}", quote_ident(table));
    let count: i64 = conn
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|e| read_failed(table, e))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

/// Quotes a catalog-provided identifier for interpolation (LESSONS #5:
/// identifiers are whitelisted by the catalog, never user input).
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn read_failed(table: &str, error: rusqlite::Error) -> PortError {
    PortError::OpenFailed {
        detail: format!("reading {table:?} failed: {error}"),
    }
}

fn no_database() -> PortError {
    PortError::Unsupported {
        reason: "no database is open".to_owned(),
    }
}

fn write_pending() -> PortError {
    PortError::Unsupported {
        reason: "SQLite writes arrive in step 13.2 (BACKLOG H36)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affinity_table_maps_declared_types() {
        let case = |declared: &str, pk: i64, autoincrement: bool| {
            field_kind(declared, pk, autoincrement, true)
        };
        assert_eq!(
            case("INTEGER", 1, true),
            FieldKind::AutoNumber(AutoKind::LongInteger)
        );
        assert_eq!(
            case("INTEGER", 1, false),
            FieldKind::Number(NumericKind::LongInteger)
        );
        assert_eq!(
            case("VARCHAR(40)", 0, false),
            FieldKind::Text { max_len: 40 }
        );
        assert_eq!(case("TEXT", 0, false), FieldKind::Text { max_len: 255 });
        assert_eq!(
            case("BLOB", 0, false),
            FieldKind::Binary(BinaryKind::OleObject)
        );
        assert_eq!(
            case("REAL", 0, false),
            FieldKind::Number(NumericKind::Double)
        );
        assert_eq!(
            case("NUMERIC", 0, false),
            FieldKind::Number(NumericKind::Decimal {
                precision: 28,
                scale: 0
            })
        );
        assert_eq!(
            case("BOOLEAN", 0, false),
            FieldKind::Boolean { tri_state: true }
        );
        assert_eq!(case("DATETIME", 0, false), FieldKind::DateTime);
        assert_eq!(case("CURRENCY", 0, false), FieldKind::Currency);
        assert_eq!(case("GUID", 0, false), FieldKind::Guid);
    }

    #[test]
    fn text_budget_falls_back_to_the_access_maximum() {
        assert_eq!(text_budget("VARCHAR(12)"), 12);
        assert_eq!(text_budget("VARCHAR(999)"), 255);
        assert_eq!(text_budget("VARCHAR(0)"), 1);
        assert_eq!(text_budget("TEXT"), 255);
        assert_eq!(text_budget("VARCHAR("), 255);
    }

    #[test]
    fn guid_text_parses_with_and_without_braces() {
        let expected = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        assert_eq!(
            parse_guid("01234567-89ab-cdef-fedc-ba9876543210"),
            Some(expected)
        );
        assert_eq!(
            parse_guid("{01234567-89ab-cdef-fedc-ba9876543210}"),
            Some(expected)
        );
        assert_eq!(parse_guid("not-a-guid"), None);
    }

    #[test]
    fn identifiers_are_quoted_and_escaped() {
        assert_eq!(quote_ident("plain"), "\"plain\"");
        assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
    }
}
