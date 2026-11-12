//! Index and trigger inventory (phase 73, step 73.2; item M90).
//!
//! `sqlite_master` plus `PRAGMA index_list`/`index_info` describe what the
//! file carries. DataCraft lists it read-only; trigger SQL is never
//! parsed, created or executed.

use datacraft_core::port::{PortError, SourceIndex, SourceTrigger};
use rusqlite::Connection;

use crate::mapping::{quote_ident, read_failed};

/// All indexes of user tables, in creation order (automatic constraint
/// indexes included: they are real index objects with a `sqlite_autoindex_`
/// name and no SQL).
pub(crate) fn indexes(conn: &Connection) -> Result<Vec<SourceIndex>, PortError> {
    let mut statement = conn
        .prepare(
            "SELECT name, tbl_name, sql FROM sqlite_master \
             WHERE type = 'index' AND tbl_name NOT LIKE 'sqlite_%' ORDER BY rowid",
        )
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut rows = statement
        .query([])
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed("sqlite_master", e))? {
        let name: String = row.get(0).map_err(|e| read_failed("sqlite_master", e))?;
        let table: String = row.get(1).map_err(|e| read_failed("sqlite_master", e))?;
        let (unique, columns) = index_details(conn, &table, &name)?;
        out.push(SourceIndex {
            automatic: name.starts_with("sqlite_autoindex_"),
            columns,
            name,
            table,
            unique,
        });
    }
    Ok(out)
}

/// Uniqueness and columns of one index.
fn index_details(
    conn: &Connection,
    table: &str,
    index: &str,
) -> Result<(bool, Vec<String>), PortError> {
    let mut unique = false;
    let mut statement = conn
        .prepare(&format!("PRAGMA index_list({})", quote_ident(table)))
        .map_err(|e| read_failed(table, e))?;
    let mut rows = statement.query([]).map_err(|e| read_failed(table, e))?;
    while let Some(row) = rows.next().map_err(|e| read_failed(table, e))? {
        let name: String = row.get(1).map_err(|e| read_failed(table, e))?;
        if name.eq_ignore_ascii_case(index) {
            let flag: i64 = row.get(2).map_err(|e| read_failed(table, e))?;
            unique = flag != 0;
            break;
        }
    }
    drop(rows);
    drop(statement);
    let mut columns = Vec::new();
    let mut statement = conn
        .prepare(&format!("PRAGMA index_info({})", quote_ident(index)))
        .map_err(|e| read_failed(index, e))?;
    let mut rows = statement.query([]).map_err(|e| read_failed(index, e))?;
    while let Some(row) = rows.next().map_err(|e| read_failed(index, e))? {
        // (seqno, cid, name) — expression indexes carry no column name.
        let name: Option<String> = row.get(2).map_err(|e| read_failed(index, e))?;
        if let Some(name) = name {
            columns.push(name);
        }
    }
    Ok((unique, columns))
}

/// All triggers of user tables, in creation order.
pub(crate) fn triggers(conn: &Connection) -> Result<Vec<SourceTrigger>, PortError> {
    let mut statement = conn
        .prepare(
            "SELECT name, tbl_name, sql FROM sqlite_master \
             WHERE type = 'trigger' AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
        )
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut rows = statement
        .query([])
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed("sqlite_master", e))? {
        let name: String = row.get(0).map_err(|e| read_failed("sqlite_master", e))?;
        let table: String = row.get(1).map_err(|e| read_failed("sqlite_master", e))?;
        let sql: Option<String> = row.get(2).map_err(|e| read_failed("sqlite_master", e))?;
        out.push(SourceTrigger {
            name,
            sql: sql.unwrap_or_default(),
            table,
        });
    }
    Ok(out)
}
