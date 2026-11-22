//! PDF report export contract (phase 97, steps 97.2/97.4; item M128).
//!
//! The writer renders exactly the preview's page sequence: one PDF page
//! per preview page, page geometry from the report, text as text through
//! an embedded Type0 font with a ToUnicode map. Deterministic bytes, a
//! precise refusal for an existing target, and the source stays untouched.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
    )
}

fn control(name: &str, kind: ControlKind, y: f32) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind,
        rect: Rect {
            x: 36.0,
            y,
            width: 300.0,
            height: 24.0,
        },
    }
}

fn ready_core(rows: Vec<Vec<FieldValue>>) -> Core {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_row_data("Customers", rows);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
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
        control: control(
            "Title",
            ControlKind::Label {
                text: "Customers".into(),
            },
            36.0,
        ),
    });
    core.dispatch(Command::AddReportControl {
        report: "Weekly".into(),
        control: control(
            "Name",
            ControlKind::TextBox {
                bound_field: Some("Name".into()),
            },
            72.0,
        ),
    });
    core
}

fn rows(count: usize) -> Vec<Vec<FieldValue>> {
    (0..count)
        .map(|index| vec![FieldValue::Text(format!("Customer {index}"))])
        .collect()
}

fn scratch(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-pdf-{}-{n}-{stem}.pdf",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn export(core: &mut Core, path: &std::path::Path) -> Vec<Event> {
    core.dispatch(Command::ExportReportPdf {
        report: "Weekly".into(),
        path: path.to_path_buf(),
    })
}

#[test]
fn the_export_renders_one_page_per_preview_page() {
    let mut core = ready_core(rows(3));
    let path = scratch("pages");
    let scratch_file = Scratch(path.clone());
    let events = export(&mut core, &scratch_file.0);
    match events.as_slice() {
        [
            Event::ReportExported {
                report,
                pages,
                path: written,
            },
        ] => {
            assert_eq!(report, "Weekly");
            assert_eq!(*pages, 3, "one page per source record");
            assert_eq!(written, &scratch_file.0);
        }
        other => panic!("expected one export event, got {other:?}"),
    }
    let pdf = std::fs::read(&scratch_file.0).expect("the file exists");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.starts_with("%PDF-"), "a PDF header");
    assert!(text.contains("/Count 3"), "the page tree counts the pages");
    assert!(
        text.contains("/MediaBox [0 0 595 842]"),
        "A4 geometry from the report"
    );
    assert!(text.contains("/Subtype /Type0"), "an embedded CID font");
    assert!(text.contains("/ToUnicode"), "a mapping back to text");
}

#[test]
fn re_exporting_identical_input_is_byte_identical() {
    let mut core = ready_core(rows(2));
    let first = scratch("det-a");
    let second = scratch("det-b");
    let _a = Scratch(first.clone());
    let _b = Scratch(second.clone());
    export(&mut core, &first);
    export(&mut core, &second);
    assert_eq!(
        std::fs::read(&first).expect("first"),
        std::fs::read(&second).expect("second"),
        "fixed metadata, stable object order"
    );
}

#[test]
fn text_stays_text_through_the_to_unicode_map() {
    let mut core = ready_core(rows(1));
    let path = scratch("text");
    let scratch_file = Scratch(path.clone());
    export(&mut core, &scratch_file.0);
    let pdf = String::from_utf8_lossy(&std::fs::read(&scratch_file.0).expect("bytes")).into_owned();
    // The CMap maps glyphs back to UTF-16BE; 'C' is 0043 and 'u' is 0075.
    let glyph = c_map_glyph_for(&pdf, "0043").expect("'C' is mapped");
    assert!(
        pdf.contains(&glyph),
        "the content stream uses the mapped glyph {glyph}"
    );
    assert!(
        c_map_glyph_for(&pdf, "0075").is_some(),
        "the label's characters are recoverable"
    );
}

/// Finds `<glyph> <utf16be…>` in the ToUnicode CMap stream and returns the
/// glyph hex for the given UTF-16BE text.
fn c_map_glyph_for(pdf: &str, utf16: &str) -> Option<String> {
    for line in pdf.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('<') else {
            continue;
        };
        let Some((glyph, tail)) = rest.split_once("> <") else {
            continue;
        };
        let target = tail.trim_end_matches('>');
        if target.eq_ignore_ascii_case(utf16) {
            return Some(glyph.to_owned());
        }
    }
    None
}

#[test]
fn an_empty_source_exports_its_one_empty_page() {
    let mut core = ready_core(Vec::new());
    let path = scratch("empty");
    let scratch_file = Scratch(path.clone());
    match export(&mut core, &scratch_file.0).as_slice() {
        [Event::ReportExported { pages, .. }] => assert_eq!(*pages, 1),
        other => panic!("expected one export event, got {other:?}"),
    }
    let pdf = std::fs::read(&scratch_file.0).expect("file");
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 1"));
}

#[test]
fn an_existing_target_is_refused_without_touching_it() {
    let mut core = ready_core(rows(1));
    let path = scratch("existing");
    std::fs::write(&path, b"foreign bytes").expect("seed the target");
    let scratch_file = Scratch(path.clone());
    let events = export(&mut core, &scratch_file.0);
    match events.as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Io);
            assert!(reason.contains("already exists"), "{reason}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&scratch_file.0).expect("target"),
        b"foreign bytes",
        "the foreign file is untouched"
    );
}

#[test]
fn unknown_reports_and_closed_databases_fail_precisely() {
    let mut core = ready_core(Vec::new());
    let path = scratch("unknown");
    let scratch_file = Scratch(path.clone());
    let events = core.dispatch(Command::ExportReportPdf {
        report: "Missing".into(),
        path: scratch_file.0.clone(),
    });
    match events.as_slice() {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert!(!scratch_file.0.exists(), "nothing is written on failure");

    let mut closed = Core::new(Box::new(MockPort::new()));
    let events = export(&mut closed, &scratch_file.0);
    match events.as_slice() {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NoDatabase),
        other => panic!("expected NoDatabase, got {other:?}"),
    }
}
