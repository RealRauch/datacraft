//! App-level command flows (steps 11.3/11.4): table creation and object
//! deletion compose existing bus commands; no logic lives in paint code.

use datacraft_core::command::Command;
use datacraft_core::event::Event;

use crate::app::App;

impl App {
    /// Creates a new table with the Access default shape (step 11.3) and
    /// opens it — the ribbon's New Table flow.
    pub fn create_table_flow(&mut self) {
        let name = next_free_name(
            self.snapshot()
                .database
                .as_ref()
                .map(|db| db.tables.iter().map(|t| t.name.to_lowercase()).collect())
                .unwrap_or_default(),
            "Table",
        );
        let events = self.dispatch(Command::CreateTable {
            definition: datacraft_core::schema::TableDef::access_default(name.clone()),
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::TableCreated { .. }))
        {
            self.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
                datacraft_core::ObjectKind::Table,
                name,
            )));
        }
    }

    /// Deletes a database object through the bus (step 11.4).
    pub fn delete_object(&mut self, kind: datacraft_core::ObjectKind, name: &str) {
        let command = match kind {
            datacraft_core::ObjectKind::Table => Command::DropTable {
                table: name.to_owned(),
            },
            datacraft_core::ObjectKind::Query => Command::DeleteQuery {
                name: name.to_owned(),
            },
            datacraft_core::ObjectKind::Form => Command::DeleteForm {
                name: name.to_owned(),
            },
            datacraft_core::ObjectKind::Report => Command::DeleteReport {
                name: name.to_owned(),
            },
        };
        self.dispatch(command);
    }

    /// Reason of the last failed command (status bar).
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

/// First free `prefixN` name (case-insensitive).
fn next_free_name(used: Vec<String>, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !used.contains(&name.to_lowercase()))
        .unwrap_or_else(|| prefix.to_owned())
}
