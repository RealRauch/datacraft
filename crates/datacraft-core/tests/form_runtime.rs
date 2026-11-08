//! Contract suite for form runtime and data binding (step 8.3, item H30).
//!
//! Forms bind to a source table; every bound field passes the schema
//! whitelist (LESSONS #5). The design document persists to a real
//! `.datacraft` file (version gate, database-name check).

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};

fn customers_def() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn opened_core() -> Core {
    let mut core = opened_db();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core
}

fn opened_db() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
    });
    core.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    core
}

fn text_box(name: &str, bound: Option<&str>) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::TextBox {
            bound_field: bound.map(str::to_owned),
        },
        rect: Rect {
            x: 8.0,
            y: 8.0,
            width: 120.0,
            height: 24.0,
        },
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

#[test]
fn set_form_source_is_whitelisted_and_observable() {
    let mut core = opened_core();
    assert_eq!(
        core.dispatch(Command::SetFormSource {
            form: "Card".into(),
            table: "Customers".into(),
        }),
        vec![Event::FormSourceSet {
            form: "Card".into(),
            table: "Customers".into(),
        }]
    );
    assert_eq!(
        core.inspect().database.unwrap().forms[0].source,
        "Customers"
    );

    let reason = failed_reason(&core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Ghost".into(),
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn bound_fields_validate_against_the_source() {
    let mut core = opened_core();
    // No source yet: any binding is premature.
    let reason = failed_reason(&core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: text_box("name_box", Some("name")),
    }));
    assert!(reason.contains("source"), "{reason}");

    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: text_box("name_box", Some("name")),
    });
    let reason = failed_reason(&core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: text_box("ghost_box", Some("ghost")),
    }));
    assert!(reason.to_lowercase().contains("ghost"), "{reason}");
}

#[test]
fn setting_a_source_validates_existing_bindings() {
    let mut core = opened_core();
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: text_box("name_box", Some("name")),
    });
    // Switching to a source without that field is refused.
    core.dispatch(Command::CreateTable {
        definition: TableDef::new("Orders", vec![FieldDef::new("total", FieldKind::Currency)]),
    });
    let reason = failed_reason(&core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Orders".into(),
    }));
    assert!(reason.contains("name"), "{reason}");
    // The original source survives the refusal.
    assert_eq!(
        core.inspect().database.unwrap().forms[0].source,
        "Customers"
    );
}

#[test]
fn the_design_document_saves_and_loads() {
    let path = std::env::temp_dir().join(format!(
        "datacraft-design-{}-{}.datacraft",
        std::process::id(),
        line!()
    ));
    let mut core = opened_core();
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    core.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: text_box("name_box", Some("name")),
    });
    core.dispatch(Command::CreateQuery {
        name: "Active".into(),
    });

    assert_eq!(
        core.dispatch(Command::SaveDesignDocument { path: path.clone() }),
        vec![Event::DesignDocumentSaved { path: path.clone() }]
    );

    // A fresh core (database without design layer) reloads it.
    let mut core = opened_db();
    assert!(core.inspect().database.unwrap().forms.is_empty());
    assert_eq!(
        core.dispatch(Command::LoadDesignDocument { path: path.clone() }),
        vec![Event::DesignDocumentLoaded {
            path: path.clone(),
            forms: 1,
            queries: 1,
            reports: 0,
        }]
    );
    let db = core.inspect().database.unwrap();
    assert_eq!(db.forms[0].name, "Card");
    assert_eq!(db.forms[0].source, "Customers");
    assert_eq!(db.queries.len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loading_rejects_foreign_and_future_documents() {
    let path = std::env::temp_dir().join(format!(
        "datacraft-design-{}-{}.datacraft",
        std::process::id(),
        line!()
    ));
    let mut core = opened_core();
    core.dispatch(Command::SaveDesignDocument { path: path.clone() });

    // A different database must not adopt the document.
    let mut other = Core::new(Box::new(MockPort::new()));
    other.dispatch(Command::OpenDatabase {
        path: PathBuf::from("other.mdb"),
        password: None,
    });
    other.dispatch(Command::CreateTable {
        definition: customers_def(),
    });
    other.dispatch(Command::CreateForm { name: "X".into() });
    let reason = failed_reason(&other.dispatch(Command::LoadDesignDocument { path: path.clone() }));
    assert!(reason.contains("shop"), "{reason}");

    // A future version is rejected precisely.
    let mut json = std::fs::read_to_string(&path).unwrap();
    json = json.replace("\"schema_version\": 1", "\"schema_version\": 99");
    std::fs::write(&path, json).unwrap();
    let reason = failed_reason(&core.dispatch(Command::LoadDesignDocument { path: path.clone() }));
    assert!(reason.contains("99"), "{reason}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_requires_an_open_database() {
    let reason = failed_reason(&Core::new(Box::new(MockPort::new())).dispatch(
        Command::SaveDesignDocument {
            path: PathBuf::from("nowhere.datacraft"),
        },
    ));
    assert!(reason.contains("no database"), "{reason}");
}

// --- sidecar failure arms (precise I/O errors, no silent swallowing) ---

#[test]
fn sidecar_io_failures_surface_as_failed_events() {
    let mut core = opened_db();
    let dir = std::env::temp_dir();
    match core
        .dispatch(Command::SaveDesignDocument { path: dir.clone() })
        .first()
    {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("write failed"), "{reason}"),
        other => panic!("{other:?}"),
    }
    match core
        .dispatch(Command::LoadDesignDocument {
            path: dir.join("datacraft-does-not-exist.datacraft"),
        })
        .first()
    {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("read failed"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn form_guards_name_the_missing_object() {
    let mut core = opened_db();
    match core
        .dispatch(Command::DeleteForm {
            name: "Ghost".into(),
        })
        .first()
    {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("not found"), "{reason}"),
        other => panic!("{other:?}"),
    }
    match core
        .dispatch(Command::SetFormSource {
            form: "Ghost".into(),
            table: "T".into(),
        })
        .first()
    {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("not found"), "{reason}"),
        other => panic!("{other:?}"),
    }
    match core
        .dispatch(Command::SetFormControlRect {
            form: "Ghost".into(),
            control: "c".into(),
            rect: datacraft_core::canvas::Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        })
        .first()
    {
        Some(Event::Failed { reason, .. }) => assert!(reason.contains("not found"), "{reason}"),
        other => panic!("{other:?}"),
    }
}
