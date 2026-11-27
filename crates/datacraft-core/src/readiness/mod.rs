//! App-level migration readiness report (phase 103, step 103.1; item
//! M134; roadmap §14).
//!
//! One read-only aggregation over the open database: every object with
//! its compatibility-ring status, the VBA transpilation estimate, the
//! phase-100 linked-table inventory, the cross-cutting unsupported list
//! and an effort summary. Read-only by construction — the analysis takes
//! `&DatabaseState`, resolves nothing, reads no rows and writes nothing.
//!
//! Classification rules (deliberately coarse, evidence-based):
//! - Local tables, sidecar queries/views/forms/reports and the
//!   executable native-query subset run with the shipped rings.
//! - Native queries classify through `native_exec::check` — the same
//!   rules execution applies (one rule, one source); the FROM tables
//!   must additionally resolve against the catalog.
//! - Native form/report definitions stay undecoded (M89): the catalog
//!   names them, the design migrates through `datacraft-import`
//!   (ADR-9 Route B).
//! - Macros are inventoried, never executed (ADR-2). VBA is extracted
//!   and statically analyzed, never executed (roadmap §3).
//! - Linked tables stay an inventory; targets are never resolved
//!   (roadmap §9 stage 0).
//! - Sidecar views classify by the presence of their SELECT (the
//!   sidecar loader does not re-validate; the bus paths do) — the
//!   engines surface SQL problems at run time, not here.

mod estimate;
mod vocabulary;

use crate::event::Event;
use crate::failure::FailureCode;
use crate::port::{NativeObjectKind, NativeQuery};
use crate::schema::TableDef;
use crate::state::DatabaseState;

use estimate::estimate_module;
pub use estimate::{MarkerHit, VbaModuleEstimate};
pub use vocabulary::{
    AppReadiness, EffortSummary, ObjectReadiness, ObjectStatus, ReportObjectKind, Ring,
    UnsupportedArea, UnsupportedFeature, VbaReadiness,
};

/// Aggregates the readiness report over an open database's state.
pub fn analyze(db: &DatabaseState) -> AppReadiness {
    let mut objects = Vec::new();
    for table in &db.tables {
        objects.push(entry(
            &table.name,
            ReportObjectKind::Table,
            Ring::Storage,
            ObjectStatus::Runs,
        ));
    }
    for link in &db.linked_tables {
        objects.push(entry(
            &link.name,
            ReportObjectKind::LinkedTable,
            Ring::Storage,
            ObjectStatus::NotCovered {
                reason: "linked table: the target is inventoried, never resolved or read \
                 (roadmap §9 stage 0)"
                    .to_owned(),
            },
        ));
    }
    for query in &db.queries {
        let status = if query.table.is_empty() {
            ObjectStatus::NotCovered {
                reason: "no source table assigned".to_owned(),
            }
        } else if !table_exists(&query.table, &db.tables) {
            ObjectStatus::NotCovered {
                reason: format!("source table {:?} is missing", query.table),
            }
        } else {
            ObjectStatus::Runs
        };
        objects.push(entry(
            &query.name,
            ReportObjectKind::Query,
            Ring::Query,
            status,
        ));
    }
    for query in &db.native_queries {
        let status = match crate::native_exec::check(query) {
            Ok(()) => match missing_table(query, &db.tables) {
                Some(name) => ObjectStatus::NotCovered {
                    reason: format!(
                        "native query {:?}: source table {name:?} is missing",
                        query.name
                    ),
                },
                None => ObjectStatus::Runs,
            },
            Err(error) => ObjectStatus::NotCovered {
                reason: error.to_string(),
            },
        };
        objects.push(entry(
            &query.name,
            ReportObjectKind::NativeQuery,
            Ring::Query,
            status,
        ));
    }
    for view in &db.views {
        let status = if view.sql.trim().is_empty() {
            ObjectStatus::NotCovered {
                reason: "no SELECT statement".to_owned(),
            }
        } else {
            ObjectStatus::Runs
        };
        objects.push(entry(
            &view.name,
            ReportObjectKind::View,
            Ring::Query,
            status,
        ));
    }
    for view in &db.native_views {
        objects.push(entry(
            &view.name,
            ReportObjectKind::NativeView,
            Ring::Query,
            ObjectStatus::Runs,
        ));
    }
    for form in &db.forms {
        objects.push(entry(
            &form.name,
            ReportObjectKind::Form,
            Ring::Runtime,
            ObjectStatus::Runs,
        ));
    }
    for object in db
        .native_objects
        .iter()
        .filter(|object| object.kind == NativeObjectKind::Form)
    {
        objects.push(entry(
            &object.name,
            ReportObjectKind::NativeForm,
            Ring::Runtime,
            ObjectStatus::NotCovered {
                reason: "native form definition is not decoded (M89) — migrate the design with \
                 datacraft-import (ADR-9)"
                    .to_owned(),
            },
        ));
    }
    for report in &db.reports {
        objects.push(entry(
            &report.name,
            ReportObjectKind::Report,
            Ring::Runtime,
            ObjectStatus::Runs,
        ));
    }
    for object in db
        .native_objects
        .iter()
        .filter(|object| object.kind == NativeObjectKind::Report)
    {
        objects.push(entry(
            &object.name,
            ReportObjectKind::NativeReport,
            Ring::Runtime,
            ObjectStatus::NotCovered {
                reason: "native report definition is not decoded (M89) — migrate the design with \
                 datacraft-import (ADR-9)"
                    .to_owned(),
            },
        ));
    }
    for macro_def in &db.macros {
        objects.push(entry(
            &macro_def.name,
            ReportObjectKind::Macro,
            Ring::Macro,
            ObjectStatus::NotCovered {
                reason: "macro inventory only — macros are never executed (ADR-2)".to_owned(),
            },
        ));
    }

    let vba_modules: Vec<VbaModuleEstimate> = db
        .vba_project
        .as_ref()
        .map(|project| project.modules.iter().map(estimate_module).collect())
        .unwrap_or_default();
    let code_lines: usize = vba_modules.iter().map(|module| module.code_lines).sum();
    let candidates: usize = vba_modules
        .iter()
        .map(|module| module.candidate_lines)
        .sum();
    let vba = VbaReadiness {
        modules: vba_modules,
        code_lines,
        transplicable_percent: (code_lines > 0).then(|| (candidates * 100 / code_lines) as u8),
    };

    let running = objects
        .iter()
        .filter(|object| object.status == ObjectStatus::Runs)
        .count();
    let native_queries_not_executable = objects
        .iter()
        .filter(|object| {
            object.kind == ReportObjectKind::NativeQuery && object.status != ObjectStatus::Runs
        })
        .count();
    let effort = EffortSummary {
        objects_total: objects.len(),
        running,
        not_covered: objects.len() - running,
        native_designs: db.native_objects.len(),
        native_queries: db.native_queries.len(),
        native_queries_not_executable,
        macros: db.macros.len(),
        linked_tables: db.linked_tables.len(),
        vba_modules: vba.modules.len(),
    };

    AppReadiness {
        objects,
        vba,
        links: db.linked_tables.clone(),
        unsupported: unsupported_features(db),
        effort,
    }
}

/// Builds one assessed object.
fn entry(name: &str, kind: ReportObjectKind, ring: Ring, status: ObjectStatus) -> ObjectReadiness {
    ObjectReadiness {
        name: name.to_owned(),
        kind,
        ring,
        status,
    }
}

/// Case-insensitive catalog lookup.
fn table_exists(name: &str, tables: &[TableDef]) -> bool {
    tables
        .iter()
        .any(|table| table.name.eq_ignore_ascii_case(name))
}

/// The first FROM table the catalog does not know.
fn missing_table<'a>(query: &'a NativeQuery, tables: &'a [TableDef]) -> Option<&'a str> {
    query
        .tables
        .iter()
        .find(|name| !table_exists(name, tables))
        .map(String::as_str)
}

/// The cross-cutting unsupported features of the open database.
fn unsupported_features(db: &DatabaseState) -> Vec<UnsupportedFeature> {
    let mut out = Vec::new();
    let mut push = |area, reason| out.push(UnsupportedFeature { area, reason });
    if !db.macros.is_empty() {
        push(
            UnsupportedArea::MacroExecution,
            "macros are inventoried read-only and never executed (ADR-2)".to_owned(),
        );
    }
    if db.vba_project.is_some() {
        push(
            UnsupportedArea::VbaExecution,
            "VBA is extracted and statically analyzed; execution stays out of scope (roadmap §3)"
                .to_owned(),
        );
    }
    if !db.linked_tables.is_empty() {
        push(
            UnsupportedArea::LinkedTables,
            "linked tables stay an inventory; their targets are never resolved or read \
             (roadmap §9)"
                .to_owned(),
        );
    }
    if !db.native_objects.is_empty() {
        push(
            UnsupportedArea::NativeDefinitions,
            "native form/report definitions are not decoded (M89); migrate the designs with \
             datacraft-import (ADR-9)"
                .to_owned(),
        );
    }
    for reason in [
        db.native_queries_error.as_ref(),
        db.macros_error.as_ref(),
        db.vba_error.as_ref(),
        db.sidecar_error.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        push(UnsupportedArea::DecodeError, reason.clone());
    }
    out
}

impl crate::core::Core {
    /// The readiness report of the open database (phase 103, step 103.1;
    /// M134) — the headless accessor beside the bus event.
    pub fn analyze_app(&self) -> Result<AppReadiness, String> {
        let Some(db) = self.state.database.as_ref() else {
            return Err("no database is open — open a database first".to_owned());
        };
        Ok(analyze(db))
    }

    pub(crate) fn analyze_app_command(&mut self) -> Vec<Event> {
        match self.analyze_app() {
            Ok(report) => vec![Event::AppAnalyzed { report }],
            Err(reason) => crate::core::failed(FailureCode::NoDatabase, reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::ViewDef;

    /// A minimal open-database state for the pure `analyze` unit tests
    /// (the bus-level shapes are pinned by `tests/readiness.rs`).
    fn state_with(views: Vec<ViewDef>) -> DatabaseState {
        DatabaseState {
            table_views: Default::default(),
            path: "unit.mock".into(),
            source: None,
            name: "unit".into(),
            format: crate::port::DatabaseFormat::Jet4,
            driver: crate::port::DriverId::new("mock"),
            capabilities: crate::port::DriverCapabilities::full(),
            tables: Vec::new(),
            relationships: Vec::new(),
            relationship_groups: None,
            queries: Vec::new(),
            forms: Vec::new(),
            reports: Vec::new(),
            views,
            native_views: Vec::new(),
            indexes: Vec::new(),
            triggers: Vec::new(),
            source_facts: Default::default(),
            native_objects: Vec::new(),
            linked_tables: Vec::new(),
            native_queries: Vec::new(),
            native_queries_error: None,
            macros: Vec::new(),
            macros_error: None,
            vba_project: None,
            vba_error: None,
            sidecar_error: None,
            lock_file: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    #[test]
    fn a_view_without_a_select_reports_not_covered() {
        // The bus refuses empty SQL, but the sidecar loader does not
        // re-validate — the defensive branch must stay honest.
        let state = state_with(vec![ViewDef::new("Broken", "  ")]);
        let report = analyze(&state);
        let object = &report.objects[0];
        assert_eq!(object.kind, ReportObjectKind::View);
        assert!(
            matches!(&object.status, ObjectStatus::NotCovered { reason } if reason.contains("SELECT")),
            "{object:?}"
        );
    }

    #[test]
    fn a_view_with_a_select_runs() {
        let state = state_with(vec![ViewDef::new("Recent", "SELECT 1")]);
        let report = analyze(&state);
        assert_eq!(report.objects[0].status, ObjectStatus::Runs);
        assert_eq!(report.effort.running, 1);
    }
}
