//! SQL-window data seam (step 13.3, item H37; split from `core.rs` for the
//! file-size gate): the shadow engine materializes the loaded tables and
//! file-owned views into a throwaway database. Source reads never write.

use crate::core::{Core, failed};
use crate::event::Event;
use crate::failure::FailureCode;

impl Core {
    /// Runs one gated SQL statement over the loaded tables in a throwaway
    /// in-memory database (step 13.3, H37) — the data seam of the SQL
    /// window. Sources are read through the port and never written.
    #[cfg(feature = "sql-engine")]
    pub fn run_sql(&self, sql: &str) -> Result<crate::sql_types::SqlResult, String> {
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

    /// Without the `sql-engine` feature the engine is not compiled in; the
    /// command still dispatches and answers a precise failure (step 31.2).
    #[cfg(not(feature = "sql-engine"))]
    pub fn run_sql(&self, _sql: &str) -> Result<crate::sql_types::SqlResult, String> {
        Err(
            "the SQL engine is not part of this build (enable the datacraft-core `sql-engine` feature)"
                .to_owned(),
        )
    }

    /// Result of the last successful `RunSql` (the UI's data source).
    pub fn sql_result(&self) -> Option<&crate::sql_types::SqlResult> {
        self.state.sql_result.as_ref()
    }

    pub(crate) fn run_sql_command(&mut self, sql: &str) -> Vec<Event> {
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
