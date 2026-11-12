//! SQLite row windows and row writes (split from `lib.rs` for the
//! file-size gate in phase 73).

use datacraft_core::port::{PortError, RowRange};
use datacraft_core::schema::{FieldKind, FieldValue};

use crate::SqlitePort;
use crate::mapping::{
    count_rows, no_database, quote_ident, read_failed, read_only, sql_value, to_field_value,
};
use crate::out_of_range;

impl SqlitePort {
    pub(super) fn read_window(
        &self,
        table: &str,
        range: RowRange,
    ) -> Result<Vec<Vec<FieldValue>>, PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        let def = self.readable(table)?;
        let count = count_rows(conn, &def.name)?;
        let (start, end) = datacraft_core::port::window_slice(table, range, count)?;
        // WITHOUT ROWID tables have no storage-order rowid: their window
        // follows the declared primary key, never an implicit order (M92).
        let order = if self
            .without_rowid
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&def.name))
        {
            let columns = crate::mapping::pk_columns(conn, &def.name)?;
            if columns.is_empty() {
                String::new()
            } else {
                format!(
                    " ORDER BY {}",
                    columns
                        .iter()
                        .map(|column| quote_ident(column))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        } else {
            String::new()
        };
        let sql = format!(
            "SELECT * FROM {}{order} LIMIT ?1 OFFSET ?2",
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

    pub(super) fn write_update(
        &mut self,
        table: &str,
        row: usize,
        values: &[FieldValue],
    ) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        self.reject_view(table)?;
        self.reject_without_rowid(table)?;
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

    pub(super) fn write_delete(&mut self, table: &str, row: usize) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        self.reject_view(table)?;
        self.reject_without_rowid(table)?;
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
}
