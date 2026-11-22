//! Read-only native SQL against the open SQLite file (phase 98, step
//! 98.1; item M129).
//!
//! The statement is the user's and runs on the writer's own engine —
//! full dialect, real indexes, native views and `EXPLAIN QUERY PLAN`.
//! Writes stay on the command bus: a statement the engine reports as
//! non-read-only (including `WITH … DELETE` and PRAGMA writes) answers a
//! precise `Unsupported` before any row is produced.

use datacraft_core::port::PortError;
use datacraft_core::schema::FieldValue;
use datacraft_core::sql_types::SqlResult;
use rusqlite::Connection;
use rusqlite::types::ValueRef;

/// Executes one read-only statement; a trailing statement separator is
/// tolerated, a second statement is refused by the prepare call.
pub(crate) fn run(conn: &Connection, sql: &str) -> Result<SqlResult, PortError> {
    let statement = sql.trim().trim_end_matches(';').trim();
    if statement.is_empty() {
        return Err(PortError::QueryFailed {
            detail: "the statement is empty".to_owned(),
        });
    }
    let mut prepared = conn.prepare(statement).map_err(query_error)?;
    if !prepared.readonly() {
        return Err(PortError::Unsupported {
            reason: "SQL writes stay on the command bus — the SQL window executes \
                     read-only statements natively (use the table commands for writes)"
                .to_owned(),
        });
    }
    let columns: Vec<String> = prepared.column_names().map(str::to_owned).collect();
    let mut rows = prepared.query([]).map_err(query_error)?;
    let mut collected = Vec::new();
    while let Some(row) = rows.next().map_err(query_error)? {
        let mut values = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            values.push(match row.get_ref(index).map_err(query_error)? {
                ValueRef::Null => FieldValue::Null,
                ValueRef::Integer(value) => FieldValue::Integer(value),
                ValueRef::Real(value) => FieldValue::Float(value),
                ValueRef::Text(bytes) => {
                    FieldValue::Text(String::from_utf8_lossy(bytes).into_owned())
                }
                ValueRef::Blob(bytes) => FieldValue::Binary(bytes.to_vec()),
            });
        }
        collected.push(values);
    }
    Ok(SqlResult {
        columns,
        rows: collected,
    })
}

fn query_error(error: rusqlite::Error) -> PortError {
    PortError::QueryFailed {
        detail: error.to_string(),
    }
}
