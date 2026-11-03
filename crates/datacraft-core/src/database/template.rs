//! Shipped-template materialization (phase 29, step 29.2, item H44) —
//! split from `database.rs` for the file-size gate.

use crate::core::failed;
use crate::event::Event;
use crate::port::DriverCapabilities;

impl crate::core::Core {
    /// Materializes a shipped template into a new file (phase 29, step
    /// 29.2, item H44). Creation plus the template's tables run through the
    /// port; any failure removes the partial file and reports a precise
    /// `Failed`. The new database is not opened. The command requires no
    /// open database: materialization opens the new file through the live
    /// port, so an open session must be closed first (M27).
    pub(crate) fn create_database_from_template(
        &mut self,
        path: std::path::PathBuf,
        template: String,
    ) -> Vec<Event> {
        if self.state.database.is_some() {
            return failed("close the open database before creating a database from a template");
        }
        if !self.port.capabilities().create_database {
            return failed(DriverCapabilities::unsupported(
                &self.port.driver_id(),
                "create database files",
            ));
        }
        let Some(template) = crate::templates::find_template(&template) else {
            let available: Vec<&str> = crate::templates::templates()
                .iter()
                .map(|entry| entry.name)
                .collect();
            return failed(format!(
                "unknown template {template:?} — available: {}",
                available.join(", ")
            ));
        };
        let format = template.format;
        if let Err(e) = self.port.create_database(&path, format) {
            return failed(e.to_string());
        }
        if let Err(e) = self.materialize_template_tables(&path, &template.tables) {
            let _ = std::fs::remove_file(&path);
            return failed(e);
        }
        vec![Event::DatabaseCreated { path, format }]
    }

    /// Writes the template's tables through the port (open, create, insert,
    /// close). Returns the error text on the first failure.
    fn materialize_template_tables(
        &mut self,
        path: &std::path::Path,
        tables: &[crate::templates::TemplateTable],
    ) -> Result<(), String> {
        if tables.is_empty() {
            return Ok(());
        }
        if let Err(e) = self.port.open(path, None) {
            return Err(e.to_string());
        }
        for table in tables {
            if let Err(e) = self.port.create_table(&table.definition) {
                self.port.close();
                return Err(e.to_string());
            }
            for row in &table.rows {
                if let Err(e) = self.port.insert_row(&table.definition.name, row) {
                    self.port.close();
                    return Err(e.to_string());
                }
            }
        }
        self.port.close();
        Ok(())
    }
}
