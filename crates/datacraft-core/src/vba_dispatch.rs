//! VBA read-only bus surface (step 17.2, item L5).
//!
//! Module source is fetched per module — the snapshot stays small and
//! agents get exactly one module per command. No execution path exists.

use crate::core::failed;
use crate::event::Event;
use crate::failure::FailureCode;
use crate::vba::{VbaModule, VbaProject};

impl crate::core::Core {
    /// The extracted VBA project of the open database, if any.
    pub fn vba_project(&self) -> Option<&VbaProject> {
        self.state.database.as_ref()?.vba_project.as_ref()
    }

    /// One module by name (case-insensitive); the viewer's data source.
    pub fn vba_module(&self, name: &str) -> Option<&VbaModule> {
        self.vba_project()?.module(name)
    }

    pub(crate) fn load_vba_module(&mut self, name: &str) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed(
                FailureCode::NoDatabase,
                "no database is open — open a database first",
            );
        };
        if let Some(reason) = &db.vba_error {
            return failed(FailureCode::Unsupported, reason.clone());
        }
        let Some(project) = db.vba_project.as_ref() else {
            return failed(
                FailureCode::NotFound,
                "the open database has no VBA project",
            );
        };
        match project.module(name) {
            Some(module) => vec![Event::VbaModuleLoaded {
                module: module.clone(),
            }],
            None => failed(
                FailureCode::NotFound,
                format!("VBA module {name:?} not found"),
            ),
        }
    }

    /// Runs the static analysis over the open project (step 17.3, L5).
    pub fn analyze_vba(&self) -> Result<crate::vba_analysis::VbaAnalysis, String> {
        let Some(db) = self.state.database.as_ref() else {
            return Err("no database is open — open a database first".to_owned());
        };
        if let Some(reason) = &db.vba_error {
            return Err(reason.clone());
        }
        let Some(project) = db.vba_project.as_ref() else {
            return Err("the open database has no VBA project".to_owned());
        };
        let tables: Vec<String> = db.tables.iter().map(|table| table.name.clone()).collect();
        let queries: Vec<String> = db
            .queries
            .iter()
            .map(|query| query.name.clone())
            .chain(db.native_queries.iter().map(|query| query.name.clone()))
            .collect();
        Ok(crate::vba_analysis::analyze(project, &tables, &queries))
    }

    pub(crate) fn analyze_vba_command(&mut self) -> Vec<Event> {
        match self.analyze_vba() {
            Ok(analysis) => vec![Event::VbaAnalyzed { analysis }],
            Err(reason) => failed(FailureCode::NotFound, reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::core::Core;
    use crate::port::MockPort;

    fn core_with_project() -> Core {
        let mut port = MockPort::new();
        port.with_vba(VbaProject {
            modules: vec![VbaModule {
                name: "Utilities".to_owned(),
                kind: crate::vba::VbaModuleKind::Standard,
                source: "Attribute VB_Name = \"Utilities\"\n".to_owned(),
            }],
        });
        Core::new(Box::new(port))
    }

    #[test]
    fn accessors_answer_none_without_an_open_database() {
        let core = core_with_project();
        assert!(core.vba_project().is_none());
        assert!(core.vba_module("Utilities").is_none());
    }

    #[test]
    fn load_reports_the_precise_failure_for_a_missing_module() {
        let mut core = core_with_project();
        core.dispatch(Command::OpenDatabase {
            path: std::path::PathBuf::from("programmability.accdb"),
            password: None,
            source: None,
        });
        assert!(core.vba_project().is_some());
        assert!(core.vba_module("utilities").is_some());
        let events = core.load_vba_module("Missing");
        assert!(
            matches!(events.as_slice(), [Event::Failed { reason, .. }] if reason.contains("Missing")),
            "{events:?}"
        );
    }
}
