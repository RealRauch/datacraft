//! SQLite row inserts (steps 13.2/28.1; split from `lib.rs` for the
//! file-size gate in phase 72).
//!
//! `insert_values` is the shared path of `insert_row` (engine-assigned
//! AutoNumbers) and `restore_row` (the image carries the identity).

use datacraft_core::port::PortError;
use datacraft_core::schema::{FieldKind, FieldValue};

use crate::SqlitePort;
use crate::mapping::{no_database, quote_ident, read_only, sql_value};

impl SqlitePort {
    /// Shared insert path (step 28.1, item U14): `allow_autonumber` is the
    /// restore exception, where the image already carries the identity.
    pub(super) fn insert_values(
        &mut self,
        table: &str,
        values: &[FieldValue],
        allow_autonumber: bool,
    ) -> Result<(), PortError> {
        let conn = self.conn.as_ref().ok_or_else(no_database)?;
        if !self.writable {
            return Err(read_only());
        }
        self.reject_view(table)?;
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
