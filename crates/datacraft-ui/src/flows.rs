//! App-level command flows (steps 11.3/11.4/11.6): table creation, object
//! deletion and the designer palettes compose existing bus commands; no
//! logic lives in paint code.

use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::event::Event;
use datacraft_core::form::ControlDef;
use datacraft_core::ids::TabId;

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
            // Macros are a read-only inventory (step 17.1) — no delete.
            datacraft_core::ObjectKind::Macro => return,
        };
        self.dispatch(command);
    }

    /// Assigns a form's or report's record source (step 11.6) — the
    /// designer palette's source selector.
    pub fn set_record_source(&mut self, kind: ObjectKind, object: &str, table: &str) {
        let command = match kind {
            ObjectKind::Form => Command::SetFormSource {
                form: object.to_owned(),
                table: table.to_owned(),
            },
            ObjectKind::Report => Command::SetReportSource {
                report: object.to_owned(),
                table: table.to_owned(),
            },
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro => return,
        };
        self.dispatch(command);
    }

    /// Adds a record-source field from the designer's field list (step
    /// 11.6) as a bound control at the next free slot.
    pub fn add_designer_field(&mut self, kind: ObjectKind, object: &str, field: &str) {
        let command = match kind {
            ObjectKind::Form => Command::AddFormField {
                form: object.to_owned(),
                field: field.to_owned(),
            },
            ObjectKind::Report => Command::AddReportField {
                report: object.to_owned(),
                field: field.to_owned(),
            },
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro => return,
        };
        self.dispatch(command);
    }

    /// Adds a toolbox control at the next free slot (step 11.6). Name,
    /// default kind and placement come from the core palette helpers.
    pub fn add_toolbox_control(&mut self, kind: ObjectKind, object: &str, entry_id: &str) {
        let Some((grid, bounds, controls)) = self.design_surface(kind, object) else {
            return;
        };
        let Some(control) =
            datacraft_core::palette::place_toolbox_control(entry_id, grid, bounds, &controls)
        else {
            return;
        };
        let command = match kind {
            ObjectKind::Form => Command::AddFormControl {
                form: object.to_owned(),
                control,
            },
            ObjectKind::Report => Command::AddReportControl {
                report: object.to_owned(),
                control,
            },
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro => return,
        };
        self.dispatch(command);
    }

    /// Grid, bounds and placed controls of a design surface.
    fn design_surface(
        &self,
        kind: ObjectKind,
        object: &str,
    ) -> Option<(f32, (f32, f32), Vec<ControlDef>)> {
        let db = self.snapshot().database.as_ref()?;
        match kind {
            ObjectKind::Form => {
                let def = db
                    .forms
                    .iter()
                    .find(|form| form.name.eq_ignore_ascii_case(object))?;
                Some((
                    def.grid,
                    (def.size.width, def.size.height),
                    def.controls.clone(),
                ))
            }
            ObjectKind::Report => {
                let def = db
                    .reports
                    .iter()
                    .find(|report| report.name.eq_ignore_ascii_case(object))?;
                Some((
                    def.grid,
                    (def.page.width, def.page.height),
                    def.controls.clone(),
                ))
            }
            ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro => None,
        }
    }

    /// Inserts the buffered new-record row of a datasheet tab (step 11.7).
    /// Cell parsing errors land in the status bar; a successful insert
    /// clears the buffer so the next record starts empty.
    pub fn insert_new_row(&mut self, tab_id: TabId, table: &str) -> bool {
        let parsed = self.table_definition(table).map(|definition| {
            let cells = self.new_row_cells(tab_id, definition.fields.len());
            (
                definition.name.clone(),
                datacraft_core::row_edit::row_values(&definition.fields, &cells),
            )
        });
        let Some((canonical, parsed)) = parsed else {
            self.last_error = Some(format!("table {table:?} not found"));
            return false;
        };
        let values = match parsed {
            Ok(values) => values,
            Err(reason) => {
                self.last_error = Some(reason);
                return false;
            }
        };
        let events = self.dispatch(Command::InsertRow {
            table: canonical,
            values,
        });
        if events
            .iter()
            .any(|event| matches!(event, Event::RowInserted { .. }))
        {
            self.clear_new_row(tab_id);
            true
        } else {
            false
        }
    }

    /// Reason of the last failed command (status bar).
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Success notice of the last command (status bar), if any.
    pub fn last_notice(&self) -> Option<&str> {
        self.last_notice.as_deref()
    }
}

/// First free `prefixN` name (case-insensitive).
fn next_free_name(used: Vec<String>, prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !used.contains(&name.to_lowercase()))
        .unwrap_or_else(|| prefix.to_owned())
}
