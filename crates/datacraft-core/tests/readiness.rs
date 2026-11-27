//! App-readiness contract (phase 103, step 103.1; item M134; roadmap
//! §14).
//!
//! `Command::AnalyzeApp` aggregates the migration readiness of the open
//! database: every object with its compatibility-ring status, the VBA
//! transpilation estimate, the linked-table inventory, the cross-cutting
//! unsupported list and an effort summary — read-only by construction.

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::macro_def::MacroDef;
use datacraft_core::port::{
    LinkKind, LinkedTable, MockPort, NativeObject, NativeObjectKind, NativeQuery,
    NativeQueryColumn, NativeQueryKind,
};
use datacraft_core::qbe::{QbeRow, QbeValue};
use datacraft_core::readiness::{
    AppReadiness, ObjectStatus, ReportObjectKind, Ring, UnsupportedArea,
};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};

fn opened(port: MockPort) -> Core {
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "readiness.mock".into(),
        password: None,
        source: None,
    });
    core
}

fn customers_port() -> MockPort {
    MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
    )])
}

fn plain() -> Core {
    opened(customers_port())
}

fn report_of(events: &[Event]) -> AppReadiness {
    match events.first() {
        Some(Event::AppAnalyzed { report }) => report.clone(),
        other => panic!("expected AppAnalyzed, got {other:?}"),
    }
}

fn object_of<'a>(
    report: &'a AppReadiness,
    kind: ReportObjectKind,
    name: &str,
) -> &'a datacraft_core::readiness::ObjectReadiness {
    report
        .objects
        .iter()
        .find(|object| object.kind == kind && object.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("no {kind:?} named {name:?} in {:?}", report.objects))
}

fn assert_status(
    report: &AppReadiness,
    kind: ReportObjectKind,
    name: &str,
    ring: Ring,
    status: &ObjectStatus,
) {
    let object = object_of(report, kind, name);
    assert_eq!(object.ring, ring, "{object:?}");
    assert_eq!(&object.status, status, "{object:?}");
}

fn not_covered(fragment: &str) -> ObjectStatus {
    ObjectStatus::NotCovered {
        reason: fragment.to_owned(),
    }
}

#[test]
fn analyze_app_requires_an_open_database() {
    let core = Core::new(Box::new(MockPort::with_tables(vec![])));
    let events = core.dispatch(Command::AnalyzeApp);
    assert!(
        matches!(&events[0], Event::Failed { code: FailureCode::NoDatabase, .. }),
        "{events:?}"
    );
    assert!(core.analyze_app().is_err());
}

#[test]
fn a_plain_source_reports_only_its_tables() {
    let mut core = plain();
    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_eq!(report.objects.len(), 1);
    assert_eq!(report.objects[0].kind, ReportObjectKind::Table);
    assert_eq!(report.objects[0].name, "Customers");
    assert_eq!(report.objects[0].ring, Ring::Storage);
    assert_eq!(report.objects[0].status, ObjectStatus::Runs);
    assert!(report.links.is_empty());
    assert!(report.unsupported.is_empty());
    assert!(report.vba.modules.is_empty());
    assert_eq!(report.vba.transplicable_percent, None);
    assert_eq!(report.effort.objects_total, 1);
    assert_eq!(report.effort.running, 1);
    assert_eq!(report.effort.not_covered, 0);
}

#[test]
fn sidecar_designs_run_in_their_rings() {
    let mut core = plain();
    core.dispatch(Command::CreateQuery {
        name: "By Name".into(),
    });
    core.dispatch(Command::SetQbeCell {
        query: "By Name".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    core.dispatch(Command::View(datacraft_core::view::ViewCommand::Create {
        name: "Recent".into(),
        sql: "SELECT Name FROM Customers".into(),
    }));
    core.dispatch(Command::CreateForm { name: "Card".into() });
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::CreateReport {
        name: "Invoice".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Invoice".into(),
        table: "Customers".into(),
    });

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_status(
        &report,
        ReportObjectKind::Query,
        "By Name",
        Ring::Query,
        &ObjectStatus::Runs,
    );
    assert_status(
        &report,
        ReportObjectKind::View,
        "Recent",
        Ring::Query,
        &ObjectStatus::Runs,
    );
    assert_status(
        &report,
        ReportObjectKind::Form,
        "Card",
        Ring::Runtime,
        &ObjectStatus::Runs,
    );
    assert_status(
        &report,
        ReportObjectKind::Report,
        "Invoice",
        Ring::Runtime,
        &ObjectStatus::Runs,
    );
}

#[test]
fn unassigned_sources_report_not_covered() {
    let mut core = plain();
    core.dispatch(Command::CreateQuery {
        name: "Empty Query".into(),
    });
    core.dispatch(Command::View(datacraft_core::view::ViewCommand::Create {
        name: "Empty View".into(),
        sql: String::new(),
    }));
    core.dispatch(Command::CreateForm { name: "Unbound".into() });

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    let query = object_of(&report, ReportObjectKind::Query, "Empty Query");
    assert!(
        matches!(&query.status, ObjectStatus::NotCovered { reason } if reason.contains("source table")),
        "{query:?}"
    );
    let view = object_of(&report, ReportObjectKind::View, "Empty View");
    assert!(
        matches!(&view.status, ObjectStatus::NotCovered { .. }),
        "{view:?}"
    );
    // An unbound form still renders in the runtime.
    assert_status(
        &report,
        ReportObjectKind::Form,
        "Unbound",
        Ring::Runtime,
        &ObjectStatus::Runs,
    );
}

fn native_select(name: &str) -> NativeQuery {
    NativeQuery {
        name: name.to_owned(),
        kind: NativeQueryKind::Select,
        tables: vec!["Customers".to_owned()],
        columns: vec![NativeQueryColumn {
            table: None,
            field: Some("Name".to_owned()),
            expression: None,
            descending: false,
        }],
        criteria: Vec::new(),
        group_by: Vec::new(),
        having: Vec::new(),
        joins: Vec::new(),
        order_by: Vec::new(),
        select_flags: 0,
        parameters: Vec::new(),
    }
}

#[test]
fn native_queries_classify_through_the_execution_subset() {
    let mut grouped = native_select("Totals");
    grouped.group_by = vec!["Name".to_owned()];
    let mut foreign = native_select("Foreign");
    foreign.tables = vec!["Missing".to_owned()];
    let mut port = customers_port();
    port.with_native_queries(vec![
        native_select("Clean"),
        NativeQuery {
            kind: NativeQueryKind::Crosstab,
            ..native_select("Pivot")
        },
        grouped,
        foreign,
    ]);
    let mut core = opened(port);

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_status(
        &report,
        ReportObjectKind::NativeQuery,
        "Clean",
        Ring::Query,
        &ObjectStatus::Runs,
    );
    for (name, fragment) in [
        ("Pivot", "crosstab"),
        ("Totals", "group by"),
        ("Foreign", "missing"),
    ] {
        let object = object_of(&report, ReportObjectKind::NativeQuery, name);
        assert_eq!(object.ring, Ring::Query, "{object:?}");
        assert!(
            matches!(&object.status, ObjectStatus::NotCovered { reason } if reason.to_ascii_lowercase().contains(fragment)),
            "{object:?}"
        );
    }
    assert_eq!(report.effort.native_queries, 4);
    assert_eq!(report.effort.native_queries_not_executable, 3);
}

#[test]
fn native_designs_macros_and_links_report_not_covered() {
    let mut port = customers_port();
    port.with_native_objects(vec![
        NativeObject {
            name: "LegacyForm".into(),
            kind: NativeObjectKind::Form,
        },
        NativeObject {
            name: "LegacyReport".into(),
            kind: NativeObjectKind::Report,
        },
    ]);
    port.with_macros(vec![MacroDef {
        name: "AutoExec".into(),
        actions: Vec::new(),
        definition_decoded: false,
    }]);
    port.with_linked_tables(vec![
        LinkedTable {
            name: "Remote".into(),
            kind: LinkKind::Odbc,
            source: None,
            connect: Some("ODBC;DSN=bench;UID=***".into()),
            foreign_name: Some("dbo.Remote".into()),
            flags: 0,
        },
        LinkedTable {
            name: "Other".into(),
            kind: LinkKind::AccessFile,
            source: Some("C:\\data\\other.mdb".into()),
            connect: None,
            foreign_name: Some("Table1".into()),
            flags: 0,
        },
    ]);
    let mut core = opened(port);

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_status(
        &report,
        ReportObjectKind::NativeForm,
        "LegacyForm",
        Ring::Runtime,
        &not_covered("datacraft-import"),
    );
    assert_status(
        &report,
        ReportObjectKind::NativeReport,
        "LegacyReport",
        Ring::Runtime,
        &not_covered("datacraft-import"),
    );
    assert_status(
        &report,
        ReportObjectKind::Macro,
        "AutoExec",
        Ring::Macro,
        &not_covered("never executed"),
    );
    for name in ["Remote", "Other"] {
        assert_status(
            &report,
            ReportObjectKind::LinkedTable,
            name,
            Ring::Storage,
            &not_covered("never resolved"),
        );
    }
    assert_eq!(report.links.len(), 2);
    assert_eq!(report.links[0].name, "Remote");
    assert_eq!(report.links[0].kind, LinkKind::Odbc);
    assert_eq!(report.effort.native_designs, 2);
    assert_eq!(report.effort.macros, 1);
    assert_eq!(report.effort.linked_tables, 2);

    for area in [
        UnsupportedArea::MacroExecution,
        UnsupportedArea::LinkedTables,
        UnsupportedArea::NativeDefinitions,
    ] {
        assert!(
            report.unsupported.iter().any(|entry| entry.area == area),
            "missing {area:?} in {:?}",
            report.unsupported
        );
    }
}

#[test]
fn the_vba_estimate_counts_candidate_lines_and_markers() {
    let mut port = customers_port();
    port.with_vba(VbaProject {
        modules: vec![
            VbaModule {
                name: "Clean".into(),
                kind: VbaModuleKind::Standard,
                source: "Attribute VB_Name = \"Clean\"\nSub A()\n  x = 1\nEnd Sub\n".into(),
            },
            VbaModule {
                name: "Win".into(),
                kind: VbaModuleKind::Standard,
                source: "Attribute VB_Name = \"Win\"\nPrivate Declare PtrSafe Function F Lib \"user32\" () As Long\nSub B()\n  On Error Resume Next\n  Set s = CreateObject(\"Shell\")\nEnd Sub\n".into(),
            },
        ],
    });
    let mut core = opened(port);

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_eq!(report.vba.modules.len(), 2);
    assert_eq!(report.vba.modules[0].name, "Clean");
    assert_eq!(report.vba.modules[0].code_lines, 4);
    assert_eq!(report.vba.modules[0].candidate_lines, 4);
    assert!(report.vba.modules[0].markers.is_empty());
    assert_eq!(report.vba.modules[1].name, "Win");
    assert_eq!(report.vba.modules[1].code_lines, 6);
    assert_eq!(report.vba.modules[1].candidate_lines, 3);
    let markers: Vec<&str> = report.vba.modules[1]
        .markers
        .iter()
        .map(|hit| hit.marker.as_str())
        .collect();
    assert_eq!(markers, ["createobject", "declare", "on error"]);
    assert_eq!(report.vba.code_lines, 10);
    assert_eq!(report.vba.transplicable_percent, Some(70));
    assert_eq!(report.effort.vba_modules, 2);
    assert!(
        report
            .unsupported
            .iter()
            .any(|entry| entry.area == UnsupportedArea::VbaExecution),
        "{:?}",
        report.unsupported
    );
}

#[test]
fn decode_failures_surface_in_the_unsupported_list() {
    let mut port = customers_port();
    port.with_native_queries_error("attribute 12 is outside the decodable subset");
    port.with_macros_error("the macro definition blob is truncated");
    port.with_vba_error("the dir stream is damaged");
    let mut core = opened(port);

    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    let reasons: Vec<&str> = report
        .unsupported
        .iter()
        .filter(|entry| entry.area == UnsupportedArea::DecodeError)
        .map(|entry| entry.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        [
            "attribute 12 is outside the decodable subset",
            "the macro definition blob is truncated",
            "the dir stream is damaged",
        ]
    );
}

#[test]
fn analyze_app_is_read_only_by_construction() {
    let mut port = customers_port();
    port.with_read_failure("this suite forbids row reads");
    let mut core = opened(port);
    let before = core.inspect();

    let report = report_of(&core.dispatch(Command::AnalyzeApp));

    assert_eq!(core.inspect(), before);
    // The classification never needed the rows the port refuses to serve.
    assert_eq!(report.effort.objects_total, 1);
    assert_eq!(report.effort.running, 1);
}

#[test]
fn the_accessor_and_the_event_carry_the_same_report() {
    let mut core = plain();
    core.dispatch(Command::CreateForm { name: "Card".into() });
    let expected = core.analyze_app().expect("the analysis succeeds");
    let report = report_of(&core.dispatch(Command::AnalyzeApp));
    assert_eq!(report, expected);
}
