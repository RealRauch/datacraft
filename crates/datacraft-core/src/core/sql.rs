//! SQL-window data seam (step 13.3, item H37; split from `core.rs` for the
//! file-size gate).
//!
//! Two surfaces behind one command (phase 98, item M129): a driver that
//! declares `native_sql` executes the statement itself (SQLite — full
//! dialect, real indexes, native views); every other source materializes
//! its tables into the throwaway shadow engine. Neither path writes the
//! source.

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;

impl Core {
    /// Runs one SQL statement over the open source: natively when the
    /// driver declares it, otherwise through the shadow engine over the
    /// rows read through the port. Sources are never written.
    pub fn run_sql(&self, sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        let Some(db) = &self.state.database else {
            return Err("no database is open".to_owned());
        };
        if db.capabilities.native_sql == crate::port::Support::Yes {
            return self
                .port
                .run_native_sql(sql)
                .map_err(|error| error.to_string());
        }
        self.run_sql_shadow(sql)
    }

    /// The shadow path: materialize the loaded tables and file-owned views
    /// into a throwaway database (step 13.3, H37).
    #[cfg(feature = "sql-engine")]
    fn run_sql_shadow(&self, sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        let Some(db) = &self.state.database else {
            return Err("no database is open".to_owned());
        };
        let mut sources = Vec::with_capacity(db.tables.len() + db.native_views.len());
        for table in &db.tables {
            let rows = self
                .port
                .read_rows(
                    &table.name,
                    crate::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .map_err(|e| format!("materializing {:?} failed: {e}", table.name))?;
            sources.push((table.clone(), rows));
        }
        // File-owned views materialize as read-only sources (M90): the SQL
        // window sees them like tables without touching the design layer.
        for view in &db.native_views {
            let rows = self
                .port
                .read_rows(
                    &view.name,
                    crate::port::RowRange {
                        start: 0,
                        len: usize::MAX,
                    },
                )
                .map_err(|e| format!("materializing view {:?} failed: {e}", view.name))?;
            sources.push((
                crate::schema::TableDef::new(&view.name, view.fields.clone()),
                rows,
            ));
        }
        crate::sql_engine::run_sql(sql, &sources)
    }

    /// Without the `sql-engine` feature the shadow engine is not compiled
    /// in; native drivers still answer through their own engine.
    #[cfg(not(feature = "sql-engine"))]
    fn run_sql_shadow(&self, _sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        Err(
            "the shadow SQL engine is not part of this build (enable the datacraft-core \
             `sql-engine` feature); this source's driver has no native SQL engine"
                .to_owned(),
        )
    }

    /// Result of the last successful `RunSql` (the UI's data source).
    pub fn sql_result(&self) -> Option<&crate::sql_types::SqlResult> {
        self.state.sql_result.as_ref()
    }

    pub(crate) fn run_sql_command(&mut self, sql: &str) -> Vec<Event> {
        // Native pushdown keeps its typed failures: a write refusal is a
        // capability boundary, a malformed statement a parse failure.
        if self
            .state
            .database
            .as_ref()
            .is_some_and(|db| db.capabilities.native_sql == crate::port::Support::Yes)
        {
            return match self.port.run_native_sql(sql) {
                Ok(result) => {
                    let rows = result.rows.len();
                    self.state.sql_result = Some(result);
                    vec![Event::SqlRan { rows }]
                }
                Err(crate::port::PortError::Unsupported { reason }) => {
                    self.state.sql_result = None;
                    failed(FailureCode::Unsupported, reason)
                }
                Err(error) => {
                    self.state.sql_result = None;
                    failed(FailureCode::Parse, error.to_string())
                }
            };
        }
        match self.run_sql(sql) {
            Ok(result) => {
                let rows = result.rows.len();
                self.state.sql_result = Some(result);
                vec![Event::SqlRan { rows }]
            }
            Err(reason) => {
                self.state.sql_result = None;
                failed(FailureCode::Parse, reason)
            }
        }
    }
}
