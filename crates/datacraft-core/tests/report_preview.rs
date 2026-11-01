//! Print-preview pagination contract (steps 10.5/15.2, items H31/H41).
//!
//! The preview runtime is a pure function: a report design acts as the
//! page template, one page per source record (ungrouped reports stay on
//! single-record detail pages; grouped/aggregate bands live in
//! `report_bands`). Layout math lives in core (Principle 4); the UI only
//! paints pages.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::report::ReportDef;
use datacraft_core::report_preview::{Band, paginate};
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

fn label(name: &str, text: &str) -> datacraft_core::form::ControlDef {
    datacraft_core::form::ControlDef {
        name: name.into(),
        kind: datacraft_core::form::ControlKind::Label { text: text.into() },
        rect: Rect {
            x: 36.0,
            y: 36.0,
            width: 200.0,
            height: 24.0,
        },
    }
}

fn bound(name: &str, field: &str) -> datacraft_core::form::ControlDef {
    datacraft_core::form::ControlDef {
        name: name.into(),
        kind: datacraft_core::form::ControlKind::TextBox {
            bound_field: Some(field.into()),
        },
        rect: Rect {
            x: 36.0,
            y: 72.0,
            width: 200.0,
            height: 24.0,
        },
    }
}

fn rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![FieldValue::Text("Alice".into()), FieldValue::Boolean(true)],
        vec![FieldValue::Text("Bob".into()), FieldValue::Boolean(false)],
    ]
}

#[test]
fn empty_source_yields_exactly_one_template_page() {
    let report = ReportDef::new("Weekly");
    let pages = paginate(&report, None, &[]).expect("renders");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].items.is_empty());
}

#[test]
fn labels_render_literal_on_every_page() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(label("header", "Sales Weekly"));
    let pages = paginate(&report, Some(&customers()), &rows()).expect("renders");
    assert_eq!(pages.len(), 2, "one page per record");
    for page in &pages {
        assert_eq!(page.band, Band::Detail);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].text, "Sales Weekly");
        assert_eq!(page.items[0].rect, report.controls[0].rect);
    }
}

#[test]
fn bound_controls_render_the_record_values() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(label("header", "Customer:"));
    report.controls.push(bound("nameBox", "Name"));
    report.controls.push(bound("activeBox", "Active"));
    let pages = paginate(&report, Some(&customers()), &rows()).expect("renders");
    let texts: Vec<String> = pages[0].items.iter().map(|i| i.text.clone()).collect();
    assert_eq!(
        texts,
        vec!["Customer:".to_owned(), "Alice".to_owned(), "Yes".to_owned()]
    );
    let texts: Vec<String> = pages[1].items.iter().map(|i| i.text.clone()).collect();
    assert_eq!(
        texts,
        vec!["Customer:".to_owned(), "Bob".to_owned(), "No".to_owned()]
    );
}

#[test]
fn unknown_bound_fields_render_empty_not_broken() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(bound("ghost", "Missing"));
    let pages = paginate(&report, Some(&customers()), &rows()).expect("renders");
    assert_eq!(pages[0].items[0].text, "");
}

#[test]
fn no_source_renders_unbound_values_only() {
    let mut report = ReportDef::new("Weekly");
    report.controls.push(label("header", "Cover Page"));
    report.controls.push(bound("nameBox", "Name"));
    let pages = paginate(&report, None, &[]).expect("renders");
    let texts: Vec<String> = pages[0].items.iter().map(|i| i.text.clone()).collect();
    assert_eq!(texts, vec!["Cover Page".to_owned(), String::new()]);
}

/// The core entry reads the source rows through the port and paginates.
#[test]
fn core_report_preview_reads_rows_through_the_port() {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data(
        "Customers",
        vec![
            vec![FieldValue::Text("Alice".into()), FieldValue::Boolean(true)],
            vec![FieldValue::Text("Bob".into()), FieldValue::Boolean(false)],
        ],
    );
    let mut core = Core::new(Box::new(port));
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
        control: bound("nameBox", "Name"),
    });
    let pages = core.report_preview("Weekly").expect("preview builds");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1].items[0].text, "Bob");

    let err = core.report_preview("Nope").unwrap_err();
    assert!(err.contains("not found"), "{err}");
}

// --- error paths (precise failures, coverage of the guard arms) ---

#[test]
fn preview_requires_an_open_database_and_an_existing_report() {
    let core = Core::new(Box::new(MockPort::new()));
    let err = core.report_preview("Weekly").unwrap_err();
    assert!(err.contains("no database"), "{err}");
}

#[test]
fn pagination_clamps_degenerate_page_geometry() {
    let mut report = ReportDef::new("Weekly");
    // Margins wider than the page clamp to a zero printable area.
    report.page.width = 50.0;
    report.page.height = 50.0;
    assert_eq!(report.page.printable_width(), 0.0);
    assert_eq!(report.page.printable_height(), 0.0);
}

#[test]
fn bound_controls_render_empty_when_rows_are_narrower_than_the_schema() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(bound("nameBox", "Name"));
    // A row shorter than the schema (schema-change prefix rows).
    let pages = paginate(&report, Some(&customers()), &[vec![FieldValue::Null]]).expect("renders");
    assert_eq!(pages[0].items[0].text, "");
}

#[test]
fn unbound_combo_and_check_controls_render_empty() {
    let mut report = ReportDef::new("Weekly");
    for (name, kind) in [
        (
            "combo",
            datacraft_core::form::ControlKind::ComboBox { bound_field: None },
        ),
        (
            "check",
            datacraft_core::form::ControlKind::CheckBox { bound_field: None },
        ),
        (
            "box",
            datacraft_core::form::ControlKind::TextBox { bound_field: None },
        ),
    ] {
        report.controls.push(datacraft_core::form::ControlDef {
            name: name.into(),
            kind,
            rect: datacraft_core::canvas::Rect {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height: 12.0,
            },
        });
    }
    let pages = paginate(&report, None, &[]).expect("renders");
    assert!(pages[0].items.iter().all(|i| i.text.is_empty()));
}
