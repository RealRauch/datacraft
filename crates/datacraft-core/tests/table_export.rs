//! Table export contract (phase 99, step 99.1; item M130; roadmap §11).
//!
//! One `ExportTable` command writes a table to a new file in two formats:
//! CSV (the datasheet's text form, RFC 4180 through the `csv` writer) and
//! a JSON document carrying the schema and typed rows. The write is
//! deterministic (byte-identical re-export, no timestamps), an existing
//! target is refused and left alone, and failures are precise — an
//! unknown table, a closed database, an unwritable path and a value JSON
//! cannot represent each answer their own code.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::export::ExportFormat;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::MockPort;
use datacraft_core::present::display_value;
use datacraft_core::schema::{
    BinaryKind, ComplexField, ComplexRecord, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};

/// Every scalar family the export has to carry.
fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("Name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("Qty", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Ratio", FieldKind::Number(NumericKind::Double)),
            FieldDef::new("Active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("Price", FieldKind::Currency),
            FieldDef::new(
                "Amount",
                FieldKind::Number(NumericKind::Decimal {
                    precision: 10,
                    scale: 2,
                }),
            ),
            FieldDef::new("When", FieldKind::DateTime),
            FieldDef::new("Note", FieldKind::Memo),
            FieldDef::new("Blob", FieldKind::Binary(BinaryKind::OleObject)),
            FieldDef::new("Tag", FieldKind::Guid),
        ],
    )
}

fn orders_rows() -> Vec<Vec<FieldValue>> {
    vec![
        vec![
            FieldValue::Text("Ada, \"the\" first".into()),
            FieldValue::Integer(3),
            FieldValue::Float(1.5),
            FieldValue::Boolean(true),
            FieldValue::Currency(12_500),
            FieldValue::Decimal(2_550),
            FieldValue::DateTime(46_332.625),
            FieldValue::Null,
            FieldValue::Binary(vec![1, 2]),
            FieldValue::Guid([0xab; 16]),
        ],
        vec![
            FieldValue::Text("Bob".into()),
            FieldValue::Integer(0),
            FieldValue::Float(-0.25),
            FieldValue::Boolean(false),
            FieldValue::Currency(-100),
            FieldValue::Decimal(-5),
            FieldValue::DateTime(0.0),
            FieldValue::Text("note\nwith break".into()),
            FieldValue::Binary(Vec::new()),
            FieldValue::Guid([0; 16]),
        ],
    ]
}

fn ready_core(rows: Vec<Vec<FieldValue>>) -> Core {
    let mut port = MockPort::with_tables(vec![orders()]);
    port.with_row_data("Orders", rows);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    core
}

fn scratch(stem: &str, extension: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-export-{}-{n}-{stem}.{extension}",
        std::process::id()
    ))
}

/// Removes the exported file when the test ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn export(core: &mut Core, format: ExportFormat, path: &std::path::Path) -> Vec<Event> {
    core.dispatch(Command::ExportTable {
        table: "Orders".into(),
        format,
        path: path.to_path_buf(),
    })
}

fn read(path: &std::path::Path) -> String {
    String::from_utf8(std::fs::read(path).expect("the exported file exists")).expect("UTF-8")
}

#[test]
fn csv_carries_a_header_row_and_the_datasheet_text() {
    let mut core = ready_core(orders_rows());
    let file = Scratch(scratch("csv", "csv"));
    match export(&mut core, ExportFormat::Csv, &file.0).as_slice() {
        [
            Event::TableExported {
                table,
                format,
                path,
                rows,
            },
        ] => {
            assert_eq!(table, "Orders");
            assert_eq!(*format, ExportFormat::Csv);
            assert_eq!(path, &file.0);
            assert_eq!(*rows, 2);
        }
        other => panic!("expected one export event, got {other:?}"),
    }
    let text = read(&file.0);
    // The csv crate's default terminator; CRLF never appears, so a
    // checkout's line-ending policy cannot change the bytes.
    assert!(!text.contains('\r'), "LF terminators: {text:?}");
    assert!(
        text.starts_with("Name,Qty,Ratio,Active,Price,Amount,When,Note,Blob,Tag\n"),
        "the header carries the field names in storage order: {text:?}"
    );
    // The comma/quote cell is quoted with doubled quotes (RFC 4180); the
    // memo's line break is quoted too, so it stays one record.
    assert!(
        text.contains("\"Ada, \"\"the\"\" first\",3,1.5,Yes,1.25,25.50,"),
        "the record keeps the datasheet text: {text:?}"
    );
    assert!(
        text.contains(",<binary: 2 bytes>,"),
        "null then the blob marker: {text:?}"
    );
    assert!(
        text.contains("abababab-abab-abab-abab-abababababab"),
        "the GUID keeps its canonical text: {text:?}"
    );
    assert!(
        text.contains("Bob,0,-0.25,No,-0.01,-0.05,"),
        "a value's own scale and the display text: {text:?}"
    );
    assert!(
        text.contains("\"note\nwith break\""),
        "memo with a break is quoted"
    );
    assert!(text.ends_with('\n'), "every record is terminated: {text:?}");
}

#[test]
fn csv_dates_use_the_datasheet_rendering() {
    let mut core = ready_core(orders_rows());
    let file = Scratch(scratch("csv-date", "csv"));
    export(&mut core, ExportFormat::Csv, &file.0);
    let text = read(&file.0);
    let expected = display_value(&FieldValue::DateTime(46_332.625));
    assert!(
        text.contains(&format!(",{expected},")),
        "the OLE date renders like the datasheet: {expected}"
    );
}

#[test]
fn json_carries_the_schema_and_typed_rows() {
    let mut core = ready_core(orders_rows());
    let file = Scratch(scratch("json", "json"));
    match export(&mut core, ExportFormat::Json, &file.0).as_slice() {
        [
            Event::TableExported {
                table,
                format,
                rows,
                ..
            },
        ] => {
            assert_eq!(table, "Orders");
            assert_eq!(*format, ExportFormat::Json);
            assert_eq!(*rows, 2);
        }
        other => panic!("expected one export event, got {other:?}"),
    }
    let doc: serde_json::Value = serde_json::from_str(&read(&file.0)).expect("valid JSON");
    assert_eq!(doc["table"], "Orders");
    // The schema reuses the wire vocabulary of the field kinds verbatim:
    // no second type name table can drift.
    let name = &doc["schema"][0];
    assert_eq!(name["name"], "Name");
    assert_eq!(name["kind"]["type"], "text");
    assert_eq!(name["kind"]["data"]["max_len"], 50);
    assert_eq!(name["required"], false);
    let amount = &doc["schema"][5];
    assert_eq!(amount["name"], "Amount");
    assert_eq!(amount["kind"]["type"], "number");
    assert_eq!(amount["kind"]["data"]["decimal"]["scale"], 2);

    let first = &doc["rows"][0];
    assert_eq!(first["Name"], "Ada, \"the\" first");
    assert_eq!(first["Qty"], 3);
    assert_eq!(first["Ratio"], 1.5);
    assert_eq!(first["Active"], true);
    assert_eq!(first["Price"], 1.25);
    assert_eq!(first["Amount"], 25.5, "the declared scale applies");
    assert_eq!(
        first["When"],
        display_value(&FieldValue::DateTime(46_332.625))
    );
    assert_eq!(first["Note"], serde_json::Value::Null);
    assert_eq!(first["Blob"]["binary"], "0102", "binary stays lossless");
    assert_eq!(first["Tag"], "abababab-abab-abab-abab-abababababab");
    assert_eq!(doc["rows"][1]["Active"], false);
    assert_eq!(doc["rows"][1]["Amount"], -0.05);
}

#[test]
fn a_complex_cell_exports_as_an_array_of_its_scalars() {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Tags",
        vec![FieldDef::new("Label", FieldKind::Complex)],
    )]);
    port.with_row_data(
        "Tags",
        vec![vec![FieldValue::Complex(vec![
            ComplexRecord {
                fields: vec![ComplexField {
                    name: "Value".into(),
                    value: FieldValue::Text("red".into()),
                }],
            },
            ComplexRecord {
                fields: vec![ComplexField {
                    name: "Value".into(),
                    value: FieldValue::Text("blue".into()),
                }],
            },
        ])]],
    );
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    let file = Scratch(scratch("complex", "json"));
    let events = core.dispatch(Command::ExportTable {
        table: "Tags".into(),
        format: ExportFormat::Json,
        path: file.0.clone(),
    });
    assert!(matches!(events.as_slice(), [Event::TableExported { .. }]));
    let doc: serde_json::Value = serde_json::from_str(&read(&file.0)).expect("valid JSON");
    assert_eq!(
        doc["rows"][0]["Label"],
        serde_json::json!(["red", "blue"]),
        "a multi-value cell becomes a JSON array"
    );
}

#[test]
fn re_exporting_the_same_input_is_byte_identical() {
    let mut core = ready_core(orders_rows());
    let first_csv = Scratch(scratch("det-csv-a", "csv"));
    let second_csv = Scratch(scratch("det-csv-b", "csv"));
    export(&mut core, ExportFormat::Csv, &first_csv.0);
    export(&mut core, ExportFormat::Csv, &second_csv.0);
    assert_eq!(
        std::fs::read(&first_csv.0).expect("first"),
        std::fs::read(&second_csv.0).expect("second")
    );
    let first_json = Scratch(scratch("det-json-a", "json"));
    let second_json = Scratch(scratch("det-json-b", "json"));
    export(&mut core, ExportFormat::Json, &first_json.0);
    export(&mut core, ExportFormat::Json, &second_json.0);
    assert_eq!(
        std::fs::read(&first_json.0).expect("first"),
        std::fs::read(&second_json.0).expect("second"),
        "no timestamps, stable field and row order"
    );
}

#[test]
fn an_existing_target_is_refused_and_left_alone() {
    let mut core = ready_core(orders_rows());
    let file = Scratch(scratch("existing", "csv"));
    std::fs::write(&file.0, b"foreign bytes").expect("seed the target");
    match export(&mut core, ExportFormat::Csv, &file.0).as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Io);
            assert!(reason.contains("already exists"), "{reason}");
        }
        other => panic!("expected a precise refusal, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&file.0).expect("target"),
        b"foreign bytes",
        "the foreign file is untouched"
    );
}

#[test]
fn an_unwritable_target_names_the_path() {
    let mut core = ready_core(orders_rows());
    let path = std::env::temp_dir().join(format!(
        "datacraft-export-missing-{}/out.csv",
        std::process::id()
    ));
    match export(&mut core, ExportFormat::Csv, &path).as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Io);
            assert!(reason.contains("out.csv"), "{reason}");
        }
        other => panic!("expected an Io failure, got {other:?}"),
    }
    assert!(!path.exists());
}

#[test]
fn unknown_tables_and_closed_databases_fail_precisely() {
    let mut core = ready_core(Vec::new());
    let file = Scratch(scratch("unknown", "json"));
    let events = core.dispatch(Command::ExportTable {
        table: "Missing".into(),
        format: ExportFormat::Json,
        path: file.0.clone(),
    });
    match events.as_slice() {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert!(!file.0.exists(), "nothing is written on failure");

    let mut closed = Core::new(Box::new(MockPort::new()));
    match export(&mut closed, ExportFormat::Csv, &file.0).as_slice() {
        [Event::Failed { code, .. }] => assert_eq!(code, &FailureCode::NoDatabase),
        other => panic!("expected NoDatabase, got {other:?}"),
    }
}

#[test]
fn an_empty_table_exports_a_header_and_no_rows() {
    let mut core = ready_core(Vec::new());
    let csv = Scratch(scratch("empty", "csv"));
    export(&mut core, ExportFormat::Csv, &csv.0);
    assert_eq!(
        read(&csv.0),
        "Name,Qty,Ratio,Active,Price,Amount,When,Note,Blob,Tag\n"
    );
    let json = Scratch(scratch("empty", "json"));
    export(&mut core, ExportFormat::Json, &json.0);
    let doc: serde_json::Value = serde_json::from_str(&read(&json.0)).expect("valid JSON");
    assert_eq!(doc["rows"], serde_json::json!([]));
    assert_eq!(doc["schema"].as_array().map(Vec::len), Some(10));
}

#[test]
fn a_value_json_cannot_carry_is_refused_precisely() {
    let mut rows = orders_rows();
    rows[0][2] = FieldValue::Float(f64::NAN);
    let mut core = ready_core(rows);
    let file = Scratch(scratch("nan", "json"));
    match export(&mut core, ExportFormat::Json, &file.0).as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Unsupported);
            assert!(reason.contains("Ratio"), "the field is named: {reason}");
        }
        other => panic!("expected an Unsupported failure, got {other:?}"),
    }
    assert!(!file.0.exists(), "no partial document is written");
}

#[test]
fn a_row_that_does_not_match_the_table_shape_is_refused() {
    let mut port = MockPort::with_tables(vec![orders()]);
    port.with_row_data("Orders", vec![vec![FieldValue::Text("short".into())]]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    let file = Scratch(scratch("ragged", "csv"));
    match export(&mut core, ExportFormat::Csv, &file.0).as_slice() {
        [Event::Failed { code, reason }] => {
            assert_eq!(code, &FailureCode::Port);
            assert!(reason.contains("1 values for 10 fields"), "{reason}");
        }
        other => panic!("expected a Port failure, got {other:?}"),
    }
    assert!(!file.0.exists(), "a ragged row never produces a document");
}

#[test]
fn the_command_round_trips_through_its_wire_form() {
    let command = Command::ExportTable {
        table: "Orders".into(),
        format: ExportFormat::Csv,
        path: PathBuf::from("out.csv"),
    };
    let wire = serde_json::to_value(&command).expect("serialises");
    assert_eq!(
        wire,
        serde_json::json!({
            "ExportTable": {"table": "Orders", "format": "csv", "path": "out.csv"}
        })
    );
    let parsed: Command = serde_json::from_value(wire).expect("deserialises");
    assert!(matches!(
        parsed,
        Command::ExportTable {
            format: ExportFormat::Csv,
            ..
        }
    ));
}
