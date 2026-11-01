//! Contract suite for the report design canvas (step 10.4, item H31).
//!
//! Reports share the form ControlKind canvas but carry print geometry:
//! an A4-portrait page with margins replaces the form's canvas size.
//! Commands, events and sidecar persistence mirror the form design
//! surface (ARCHITECTURE §7, H31 scope).

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_core::{ObjectId, ObjectKind};

fn customers_table() -> TableDef {
    let mut field = FieldDef::new("Name", FieldKind::Text { max_len: 50 });
    field.properties.required = false;
    TableDef::new("Customers", vec![field])
}

fn opened_with_report() -> Core {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers_table()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    core
}

fn label(name: &str, text: &str, x: f32, y: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: text.into() },
        rect: Rect {
            x,
            y,
            width: 100.0,
            height: 24.0,
        },
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn create_report_defaults_to_print_geometry() {
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![customers_table()])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    let events = core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    assert_eq!(
        events,
        vec![Event::ReportCreated {
            name: "Weekly".into()
        }]
    );
    let report = core.inspect().database.unwrap().reports[0].clone();
    assert_eq!(report.name, "Weekly");
    assert_eq!(report.page.width, 595.0, "A4 portrait width in points");
    assert_eq!(report.page.height, 842.0);
    assert_eq!(report.page.margin_left, 36.0, "half-inch margins");
    assert_eq!(report.page.margin_right, 36.0);
    assert_eq!(report.page.margin_top, 36.0);
    assert_eq!(report.page.margin_bottom, 36.0);
    assert_eq!(report.grid, 8.0);
    assert!(report.controls.is_empty());
}

#[test]
fn report_names_are_unique_case_insensitively() {
    let mut core = opened_with_report();
    let reason = failed_reason(&core.dispatch(Command::CreateReport {
        name: "weekly".into(),
    }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn report_controls_snap_and_bound_check_against_the_page() {
    let mut core = opened_with_report();
    let events = core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("title", "Sales", 37.0, 5.0),
    });
    assert_eq!(
        events,
        vec![Event::ReportControlAdded {
            report: "Weekly".into(),
            control: "title".into(),
        }]
    );
    let report = core.inspect().database.unwrap().reports[0].clone();
    assert_eq!(report.controls[0].rect.x, 40.0);
    assert_eq!(report.controls[0].rect.y, 8.0);

    // A control past the page edge is rejected precisely.
    let reason = failed_reason(&core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("wide", "off-page", 560.0, 0.0),
    }));
    assert!(reason.contains("bounds"), "{reason}");
}

#[test]
fn report_source_whitelists_and_revalidates_bindings() {
    let mut core = opened_with_report();
    // Unknown table is rejected.
    let reason = failed_reason(&core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Nope".into(),
    }));
    assert!(reason.contains("not found"), "{reason}");
    // A bound control without a source is rejected at placement time.
    let bound = ControlDef {
        name: "nameBox".into(),
        kind: ControlKind::TextBox {
            bound_field: Some("Name".into()),
        },
        rect: Rect {
            x: 0.0,
            y: 40.0,
            width: 100.0,
            height: 24.0,
        },
    };
    let reason = failed_reason(&core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: bound.clone(),
    }));
    assert!(reason.contains("source"), "{reason}");
    // With a valid source the same control is accepted, and the
    // assignment is observable.
    core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: bound,
    });
    let events = core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Customers".into(),
    });
    assert_eq!(
        events,
        vec![Event::ReportSourceSet {
            report: "Weekly".into(),
            table: "Customers".into(),
        }]
    );
}

#[test]
fn delete_report_and_move_control_behave_like_forms() {
    let mut core = opened_with_report();
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("title", "Sales", 0.0, 0.0),
    });
    let events = core.dispatch(Command::SetReportControlRect {
        report: "Weekly".into(),
        control: "title".into(),
        rect: Rect {
            x: 100.0,
            y: 50.0,
            width: 100.0,
            height: 24.0,
        },
    });
    assert_eq!(
        events,
        vec![Event::ReportControlRectSet {
            report: "Weekly".into(),
            control: "title".into(),
        }]
    );
    let events = core.dispatch(Command::DeleteReportControl {
        report: "Weekly".into(),
        control: "title".into(),
    });
    assert_eq!(
        events,
        vec![Event::ReportControlDropped {
            report: "Weekly".into(),
            control: "title".into(),
        }]
    );
    let events = core.dispatch(Command::DeleteReport {
        name: "Weekly".into(),
    });
    assert_eq!(
        events,
        vec![Event::ReportDropped {
            name: "Weekly".into()
        }]
    );
    let reason = failed_reason(&core.dispatch(Command::DeleteReport {
        name: "Weekly".into(),
    }));
    assert!(reason.contains("not found"), "{reason}");
}

#[test]
fn reports_open_as_document_tabs() {
    let mut core = opened_with_report();
    let events = core.dispatch(Command::OpenObject(ObjectId::new(
        ObjectKind::Report,
        "Weekly",
    )));
    assert!(
        events.iter().any(
            |e| matches!(e, Event::TabOpened { object, .. } if object.kind == ObjectKind::Report)
        ),
        "{events:?}"
    );
    let snapshot = core.inspect();
    let tab = &snapshot.tabs[0];
    assert_eq!(tab.object_kind, "report");
    assert_eq!(tab.ui_test_id, "workspace.tab.report.weekly");
}

#[test]
fn sidecar_round_trips_reports() {
    let mut core = opened_with_report();
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("title", "Sales", 0.0, 0.0),
    });
    let path = std::env::temp_dir().join("datacraft-report-sidecar.datacraft");
    core.dispatch(Command::SaveDesignDocument { path: path.clone() });

    let mut fresh = Core::new(Box::new(MockPort::with_tables(vec![customers_table()])));
    fresh.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    let events = fresh.dispatch(Command::LoadDesignDocument { path });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::DesignDocumentLoaded { reports: 1, .. })),
        "{events:?}"
    );
    let loaded = fresh.inspect().database.unwrap().reports;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "Weekly");
    assert_eq!(loaded[0].controls.len(), 1);
}

#[test]
fn snapshot_lists_reports_and_derives_navigation_ids() {
    let core = opened_with_report();
    let snapshot = core.inspect();
    let reports = &snapshot.database.as_ref().unwrap().reports;
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].name, "Weekly");
    // The nav pane derives the report node id from the ObjectId.
    assert_eq!(
        ObjectId::new(ObjectKind::Report, "Weekly").ui_test_id(),
        "node.report.weekly"
    );
}

// --- error paths (coverage gate: every reducer answers precisely) ---

fn failed_of(events: Vec<Event>) -> String {
    match events.first() {
        Some(Event::Failed { reason }) => reason.clone(),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn report_reducers_reject_precisely_without_a_database() {
    let mut core = Core::new(Box::new(MockPort::new()));
    let reason = failed_of(core.dispatch(Command::CreateReport { name: "R".into() }));
    assert!(reason.contains("no database"), "{reason}");
    let reason = failed_of(core.dispatch(Command::DeleteReport { name: "R".into() }));
    assert!(reason.contains("no database"), "{reason}");
    let reason = failed_of(core.dispatch(Command::AddReportControl {
        report: "R".into(),
        control: label("a", "b", 0.0, 0.0),
    }));
    assert!(reason.contains("no database"), "{reason}");
    let reason = failed_of(core.dispatch(Command::SetReportSource {
        report: "R".into(),
        table: "T".into(),
    }));
    assert!(reason.contains("no database"), "{reason}");
}

#[test]
fn create_report_rejects_empty_names() {
    let mut core = opened_with_report();
    let reason = failed_of(core.dispatch(Command::CreateReport { name: "   ".into() }));
    assert!(reason.contains("empty"), "{reason}");
}

#[test]
fn report_control_errors_name_the_missing_object() {
    let mut core = opened_with_report();
    let reason = failed_of(core.dispatch(Command::AddReportControl {
        report: "Ghost".into(),
        control: label("a", "b", 0.0, 0.0),
    }));
    assert!(reason.contains("not found"), "{reason}");
    let reason = failed_of(core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("  ", "b", 0.0, 0.0),
    }));
    assert!(reason.contains("empty"), "{reason}");
    let reason = failed_of(core.dispatch(Command::SetReportControlRect {
        report: "Weekly".into(),
        control: "ghost".into(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
    }));
    assert!(reason.contains("not found"), "{reason}");
    let reason = failed_of(core.dispatch(Command::DeleteReportControl {
        report: "Weekly".into(),
        control: "ghost".into(),
    }));
    assert!(reason.contains("not found"), "{reason}");
    let reason = failed_of(core.dispatch(Command::DeleteReport {
        name: "Ghost".into(),
    }));
    assert!(reason.contains("not found"), "{reason}");
    let reason = failed_of(core.dispatch(Command::SetReportSource {
        report: "Ghost".into(),
        table: "Customers".into(),
    }));
    assert!(reason.contains("not found"), "{reason}");
}

#[test]
fn report_source_revalidates_existing_bindings() {
    let empty = TableDef::new(
        "Empty",
        vec![FieldDef::new(
            "Other",
            FieldKind::Number(datacraft_core::schema::NumericKind::LongInteger),
        )],
    );
    let mut core = Core::new(Box::new(MockPort::with_tables(vec![
        customers_table(),
        empty,
    ])));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: {
            let mut c = label("nameBox", "", 0.0, 0.0);
            c.kind = ControlKind::TextBox {
                bound_field: Some("Name".into()),
            };
            c
        },
    });
    // Re-pointing at a table without the bound field is refused.
    let reason = failed_of(core.dispatch(Command::SetReportSource {
        report: "Weekly".into(),
        table: "Empty".into(),
    }));
    assert!(reason.contains("rebind"), "{reason}");
}

// --- remaining reducer error arms (precise, one behavior per test) ---

#[test]
fn duplicate_report_control_names_are_rejected_case_insensitively() {
    let mut core = opened_with_report();
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("title", "one", 0.0, 0.0),
    });
    let reason = failed_of(core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("TITLE", "two", 40.0, 0.0),
    }));
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn moving_a_report_control_past_the_page_edge_is_rejected() {
    let mut core = opened_with_report();
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: label("title", "one", 0.0, 0.0),
    });
    let reason = failed_of(core.dispatch(Command::SetReportControlRect {
        report: "Weekly".into(),
        control: "title".into(),
        rect: Rect {
            x: 590.0,
            y: 830.0,
            width: 100.0,
            height: 24.0,
        },
    }));
    assert!(reason.contains("bounds"), "{reason}");
}
