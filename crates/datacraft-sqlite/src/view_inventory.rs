//! File-owned view inventory (phase 73, step 73.1; item M90).
//!
//! `sqlite_master type='view'` is the source: the file owns the SQL and the
//! columns, DataCraft never writes through a view. Internal `sqlite_*`
//! names stay hidden like they are in the table catalog.

use datacraft_core::port::{NativeView, PortError};
use rusqlite::Connection;

use crate::mapping::{read_failed, table_info};

/// All non-internal views, in creation order.
pub(crate) fn collect(conn: &Connection) -> Result<Vec<NativeView>, PortError> {
    let mut statement = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master \
             WHERE type = 'view' AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
        )
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut rows = statement
        .query([])
        .map_err(|e| read_failed("sqlite_master", e))?;
    let mut views = Vec::new();
    while let Some(row) = rows.next().map_err(|e| read_failed("sqlite_master", e))? {
        let name: String = row.get(0).map_err(|e| read_failed("sqlite_master", e))?;
        let sql: Option<String> = row.get(1).map_err(|e| read_failed("sqlite_master", e))?;
        views.push(NativeView {
            fields: table_info(conn, &name, false)?,
            name,
            sql: sql.unwrap_or_default(),
        });
    }
    Ok(views)
}

/// Precise write rejection for a file-owned view.
pub(crate) fn read_only(name: &str) -> PortError {
    PortError::Unsupported {
        reason: format!("{name:?} is a native view and read-only"),
    }
}
