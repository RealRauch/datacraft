//! Index and trigger inventory (phase 73, step 73.2; item M90).
//!
//! `sqlite_master` plus `PRAGMA index_list`/`index_info` describe what the
//! file carries. DataCraft lists it read-only; trigger SQL is never
//! parsed, created or executed.

use datacraft_core::port::{PortError, SourceFact, SourceFacts, SourceIndex, SourceTrigger};
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

/// Driver facts plus the bounded `quick_check(1)` verdict (M91). The
/// check reports at most one error; a healthy file answers `"ok"`.
pub(crate) fn facts(conn: &Connection) -> SourceFacts {
    let mut facts = Vec::new();
    if let Some(version) = query_text(conn, "SELECT sqlite_version()") {
        facts.push(SourceFact {
            name: "sqlite_version".into(),
            value: version,
        });
    }
    for (name, pragma) in [
        ("page_size", "page_size"),
        ("journal_mode", "journal_mode"),
        ("encoding", "encoding"),
        ("user_version", "user_version"),
    ] {
        if let Some(value) = query_text(conn, &format!("PRAGMA {pragma}")) {
            facts.push(SourceFact {
                name: name.into(),
                value,
            });
        }
    }
    SourceFacts {
        facts,
        health: quick_check(conn),
    }
}

/// `Some(reason)` when `quick_check(1)` does not answer `"ok"`.
fn quick_check(conn: &Connection) -> Option<String> {
    let outcome = (|| -> Result<Vec<String>, rusqlite::Error> {
        let mut statement = conn.prepare("PRAGMA quick_check(1)")?;
        let mut rows = statement.query([])?;
        let mut messages = Vec::new();
        while let Some(row) = rows.next()? {
            messages.push(row.get::<_, String>(0)?);
        }
        Ok(messages)
    })();
    match outcome {
        Ok(messages) if messages == ["ok"] => None,
        Ok(messages) if messages.is_empty() => Some("quick_check: no verdict".to_owned()),
        Ok(messages) => Some(format!("quick_check: {}", messages.join("; "))),
        Err(error) => Some(format!("quick_check: {error}")),
    }
}

/// First column of the first row as text (integers and text alike).
fn query_text(conn: &Connection, sql: &str) -> Option<String> {
    conn.query_row(sql, [], |row| {
        use rusqlite::types::ValueRef;
        Ok(match row.get_ref(0)? {
            ValueRef::Null => String::new(),
            ValueRef::Integer(value) => value.to_string(),
            ValueRef::Real(value) => value.to_string(),
            ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            ValueRef::Blob(bytes) => format!("{} bytes", bytes.len()),
        })
    })
    .ok()
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
