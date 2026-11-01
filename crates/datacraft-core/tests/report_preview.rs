//! Print-preview pagination contract (step 10.5, item H31).
//!
//! The preview runtime is a pure function: a report design acts as the
//! page template, one page per source record (single-record pages —
//! grouped/aggregate bands stay deferred to the expression service, M1).
//! Layout math lives in core (Principle 4); the UI only paints pages.

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::report::ReportDef;
use datacraft_core::report_preview::paginate;
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
    let pages = paginate(&report, None, &[]);
    assert_eq!(pages.len(), 1);
    assert!(pages[0].items.is_empty());
}

#[test]
fn labels_render_literal_on_every_page() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(label("header", "Sales Weekly"));
    let pages = paginate(&report, Some(&customers()), &rows());
    assert_eq!(pages.len(), 2, "one page per record");
    for page in &pages {
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
    let pages = paginate(&report, Some(&customers()), &rows());
    let texts: Vec<String> = pages[0].items.iter().map(|i| i.text.clone()).collect();
    assert_eq!(texts, vec!["Customer:".to_owned(), "Alice".to_owned(), "Yes".to_owned()]);
    let texts: Vec<String> = pages[1].items.iter().map(|i| i.text.clone()).collect();
    assert_eq!(texts, vec!["Customer:".to_owned(), "Bob".to_owned(), "No".to_owned()]);
}

#[test]
fn unknown_bound_fields_render_empty_not_broken() {
    let mut report = ReportDef::new("Weekly");
    report.source = "Customers".into();
    report.controls.push(bound("ghost", "Missing"));
    let pages = paginate(&report, Some(&customers()), &rows());
    assert_eq!(pages[0].items[0].text, "");
}

#[test]
fn no_source_renders_unbound_values_only() {
    let mut report = ReportDef::new("Weekly");
    report.controls.push(label("header", "Cover Page"));
    report.controls.push(bound("nameBox", "Name"));
    let pages = paginate(&report, None, &[]);
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
    });
    core.dispatch(Command::CreateReport { name: "Weekly".into() });
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
