//! QBE reducers (steps 7.2/11.5; split from `core.rs` to keep every file
//! under the size gate — same pattern as `forms.rs`).

use crate::ObjectKind;
use crate::core::failed;
use crate::event::Event;
use crate::ids::TabId;
use crate::port::PortError;

impl crate::core::Core {
    pub(crate) fn create_query(&mut self, name: &str) -> Vec<Event> {
        if self.state.database.is_none() {
            return failed("no database is open — open a database first");
        }
        if name.trim().is_empty() {
            return failed(
                crate::schema::SchemaError::InvalidName {
                    detail: "names must not be empty or whitespace-only".to_owned(),
                }
                .to_string(),
            );
        }
        let db = self.state.database.as_ref().expect("checked above");
        if db
            .queries
            .iter()
            .any(|q| q.name.to_lowercase() == name.to_lowercase())
        {
            return failed(format!("duplicate query name {name:?}"));
        }
        let canonical = db
            .tables
            .iter()
            .find(|t| t.name.to_lowercase() == name.to_lowercase())
            .map(|t| t.name.clone())
            .unwrap_or_else(|| name.to_owned());
        self.state
            .database
            .as_mut()
            .expect("checked above")
            .queries
            .push(crate::qbe::QueryDef::new(&canonical, ""));
        vec![Event::QueryCreated { name: canonical }]
    }

    pub(crate) fn set_qbe_cell(
        &mut self,
        query: &str,
        row: crate::qbe::QbeRow,
        col: usize,
        value: crate::qbe::QbeValue,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .queries
            .iter_mut()
            .find(|q| q.name.to_lowercase() == query.to_lowercase())
        else {
            return failed(format!("query {query:?} not found"));
        };
        // Whitelists (LESSONS #5): the table against the database, fields
        // against their source table.
        let mut value = value;
        match row {
            crate::qbe::QbeRow::Table => {
                let crate::qbe::QbeValue::Text(table) = &value else {
                    return failed("the Table row takes a name");
                };
                if !db
                    .tables
                    .iter()
                    .any(|t| t.name.to_lowercase() == table.to_lowercase())
                {
                    return failed(PortError::TableNotFound(table.clone()).to_string());
                }
                if let crate::qbe::QbeValue::Text(chosen) = &mut value
                    && let Some(matched) = db
                        .tables
                        .iter()
                        .find(|tbl| tbl.name.to_lowercase() == chosen.to_lowercase())
                {
                    *chosen = matched.name.clone();
                }
            }
            crate::qbe::QbeRow::Field => {
                let crate::qbe::QbeValue::Text(field) = &value else {
                    return failed("the Field row takes a name");
                };
                let Some(source) = db
                    .tables
                    .iter()
                    .find(|t| t.name.to_lowercase() == def.table.to_lowercase())
                else {
                    return failed("set the source table first");
                };
                if !source
                    .fields
                    .iter()
                    .any(|f| f.name.to_lowercase() == field.to_lowercase())
                {
                    return failed(format!("field {field:?} not found in {:?}", source.name));
                }
                value = crate::qbe::QbeValue::Text(
                    source
                        .fields
                        .iter()
                        .find(|f| f.name.to_lowercase() == field.to_lowercase())
                        .expect("checked above")
                        .name
                        .clone(),
                );
            }
            _ => {}
        }
        match def.set_cell(row, col, value) {
            Err(e) => failed(e.to_string()),
            Ok(()) => vec![Event::QbeCellSet {
                query: def.name.clone(),
            }],
        }
    }

    pub(crate) fn clear_qbe_criteria(&mut self, query: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(def) = db
            .queries
            .iter_mut()
            .find(|q| q.name.eq_ignore_ascii_case(query))
        else {
            return failed(format!("query {query:?} not found"));
        };
        for field in &mut def.fields {
            field.criteria.clear();
        }
        vec![Event::QbeCriteriaCleared {
            query: def.name.clone(),
        }]
    }

    pub(crate) fn delete_query(&mut self, name: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_mut() else {
            return failed("no database is open — open a database first");
        };
        let Some(pos) = db
            .queries
            .iter()
            .position(|q| q.name.to_lowercase() == name.to_lowercase())
        else {
            return failed(format!("query {name:?} not found"));
        };
        let canonical = db.queries.remove(pos).name;
        let dependent: Vec<TabId> = self
            .state
            .tabs
            .iter()
            .filter(|tab| {
                tab.object.kind == ObjectKind::Query
                    && tab.object.name.to_lowercase() == canonical.to_lowercase()
            })
            .map(|tab| tab.id)
            .collect();
        let mut events: Vec<Event> = Vec::with_capacity(dependent.len() + 1);
        for tab_id in dependent {
            self.remove_tab(tab_id);
            events.push(Event::TabClosed { tab_id });
        }
        events.push(Event::QueryDropped { name: canonical });
        events
    }

    pub(crate) fn run_query_command(&mut self, query: &str) -> Vec<Event> {
        let canonical = self
            .state
            .database
            .as_ref()
            .and_then(|db| {
                db.queries
                    .iter()
                    .find(|q| q.name.eq_ignore_ascii_case(query))
            })
            .map(|q| q.name.clone());
        match self.run_query(query) {
            Ok(result) => vec![Event::QueryRan {
                query: canonical.unwrap_or_else(|| query.to_owned()),
                rows: result.rows.len(),
            }],
            Err(reason) => failed(reason),
        }
    }
}
