//! App-shell contract tests (steps 5.x through 17.x), moved out of the
//! binary root so the UI crate also builds as a library for the headless
//! kittest suite.

use crate::app::App;

#[test]
fn app_starts_with_a_closed_visible_shell() {
    let app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    assert!(app.snapshot().database.is_none());
    assert!(app.snapshot().nav_pane_visible);
    assert!(app.snapshot().tabs.is_empty());
}

#[test]
fn app_runs_sql_and_caches_the_result() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.set_sql_text("SELECT 1 AS one");
    app.run_sql();
    assert!(app.sql_error().is_none(), "{:?}", app.sql_error());
    let result = app.sql_result().expect("result is cached");
    assert_eq!(
        result.rows[0][0],
        datacraft_core::schema::FieldValue::Integer(1)
    );
}

#[test]
fn app_caches_sql_errors_precisely() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.set_sql_text("DELETE FROM nothing");
    app.run_sql();
    assert!(app.sql_result().is_none());
    assert!(
        app.sql_error().unwrap_or_default().contains("only SELECT"),
        "{:?}",
        app.sql_error()
    );
}

#[test]
fn the_new_table_flow_creates_and_opens_the_default_table() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.create_table_flow();
    let snapshot = app.snapshot();
    let tables = &snapshot.database.as_ref().expect("database").tables;
    assert_eq!(tables[0].name, "Table1");
    assert_eq!(tables[0].fields.len(), 2);
    assert!(
        snapshot
            .tabs
            .iter()
            .any(|tab| tab.object_kind == "table" && tab.object_name == "Table1"),
        "{:?}",
        snapshot.tabs
    );
    assert!(app.last_error().is_none(), "{:?}", app.last_error());
}

#[test]
fn delete_flows_dispatch_and_surface_precise_errors() {
    use datacraft_core::ObjectKind;
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    // The delete flow confirms natively first (step 38.3, item M43); the
    // scripted host accepts both confirmations.
    app.set_dialogs(Box::new(
        crate::dialogs::ScriptedDialogs::new()
            .confirm(true)
            .confirm(true),
    ));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.dispatch(datacraft_core::command::Command::CreateForm {
        name: "Card".into(),
    });
    app.delete_object(ObjectKind::Form, "Card");
    assert!(
        app.snapshot()
            .database
            .as_ref()
            .expect("database")
            .forms
            .is_empty()
    );
    app.delete_object(ObjectKind::Form, "Missing");
    assert!(
        app.last_error().unwrap_or_default().contains("not found"),
        "{:?}",
        app.last_error()
    );
}

#[test]
fn opening_a_database_prefills_the_sidecar_path_and_saving_reports_a_notice() {
    // The status notices are UI text: pin the language while asserting.
    let _guard = crate::i18n::lang_lock();
    crate::i18n::set_lang(crate::i18n::Lang::English);

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    assert_eq!(app.sidecar_path, "shop.datacraft");

    let dir = std::env::temp_dir().join(format!("datacraft-sidecar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("shop.datacraft");
    app.sidecar_path = path.to_string_lossy().into_owned();
    app.dispatch(datacraft_core::command::Command::CreateForm {
        name: "Card".into(),
    });
    app.dispatch(datacraft_core::command::Command::SaveDesignDocument { path: path.clone() });
    assert!(app.last_error().is_none(), "{:?}", app.last_error());
    assert!(path.exists(), "sidecar written");
    assert!(
        app.last_notice()
            .unwrap_or_default()
            .contains("Design saved"),
        "{:?}",
        app.last_notice()
    );

    app.dispatch(datacraft_core::command::Command::DeleteForm {
        name: "Card".into(),
    });
    app.dispatch(datacraft_core::command::Command::LoadDesignDocument { path: path.clone() });
    assert!(
        app.last_notice()
            .unwrap_or_default()
            .contains("Design loaded"),
        "{:?}",
        app.last_notice()
    );
    assert_eq!(app.forms().len(), 1);

    // A failure clears the previous success notice.
    app.dispatch(datacraft_core::command::Command::LoadDesignDocument {
        path: dir.join("missing.datacraft"),
    });
    assert!(app.last_error().is_some());
    assert!(app.last_notice().is_none(), "{:?}", app.last_notice());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_designer_palette_places_fields_and_toolbox_controls() {
    use datacraft_core::ObjectKind;
    use datacraft_core::form::ControlKind;
    use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.dispatch(datacraft_core::command::Command::CreateTable {
        definition: TableDef::new(
            "Customers",
            vec![
                FieldDef::new("name", FieldKind::Text { max_len: 50 }),
                FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
            ],
        ),
    });
    app.dispatch(datacraft_core::command::Command::CreateForm {
        name: "Card".into(),
    });

    app.set_record_source(ObjectKind::Form, "Card", "Customers");
    app.add_designer_field(ObjectKind::Form, "Card", "name");
    app.add_toolbox_control(ObjectKind::Form, "Card", "button");
    let form = app.snapshot().database.as_ref().expect("db").forms[0].clone();
    assert_eq!(form.controls.len(), 2);
    assert_eq!(
        form.controls[0].kind,
        ControlKind::TextBox {
            bound_field: Some("name".into())
        }
    );
    assert_eq!(
        form.controls[1].kind,
        ControlKind::Button {
            caption: "Button".into()
        }
    );
    assert!(app.last_error().is_none(), "{:?}", app.last_error());

    app.dispatch(datacraft_core::command::Command::CreateReport {
        name: "List".into(),
    });
    app.set_record_source(ObjectKind::Report, "List", "Customers");
    app.add_designer_field(ObjectKind::Report, "List", "active");
    let report = app.snapshot().database.as_ref().expect("db").reports[0].clone();
    assert_eq!(
        report.controls[0].kind,
        ControlKind::CheckBox {
            bound_field: Some("active".into())
        }
    );

    // The palette refuses a source outside the schema whitelist.
    app.set_record_source(ObjectKind::Form, "Card", "Ghost");
    assert!(
        app.last_error().unwrap_or_default().contains("Ghost"),
        "{:?}",
        app.last_error()
    );
}

#[test]
fn the_datasheet_new_row_inserts_through_the_bus() {
    use datacraft_core::ObjectId;
    use datacraft_core::ObjectKind;
    use datacraft_core::schema::{
        AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
    };

    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.dispatch(datacraft_core::command::Command::CreateTable {
        definition: TableDef::new(
            "Customers",
            vec![
                FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                FieldDef::new("name", FieldKind::Text { max_len: 20 }),
                FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
            ],
        ),
    });
    app.dispatch(datacraft_core::command::Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Customers",
    )));
    let tab = app.snapshot().tabs[0].id;

    // The buffer pads to the column count.
    assert_eq!(app.new_row_cells(tab, 3), vec!["", "", ""]);
    app.set_new_row_cells(tab, vec![String::new(), "Ada".into(), "1.5".into()]);
    assert!(app.insert_new_row(tab, "Customers"));
    let rows = app.rows_for(tab).expect("rows reloaded");
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][2], FieldValue::Float(1.5));
    assert_eq!(app.new_row_cells(tab, 3), vec!["", "", ""]);

    // A mistyped cell names the field instead of inserting.
    app.set_new_row_cells(tab, vec![String::new(), "Grace".into(), "abc".into()]);
    assert!(!app.insert_new_row(tab, "Customers"));
    assert!(
        app.last_error().unwrap_or_default().contains("\"score\""),
        "{:?}",
        app.last_error()
    );
    assert_eq!(app.rows_for(tab).map(Vec::len), Some(1));
}

#[test]
fn dispatch_refreshes_the_snapshot() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.dispatch(datacraft_core::command::Command::ToggleNavPane);
    assert!(!app.snapshot().nav_pane_visible);
    app.dispatch(datacraft_core::command::Command::ToggleNavPane);
    assert!(app.snapshot().nav_pane_visible);
}

#[test]
fn a_password_required_open_raises_the_prompt_and_retries() {
    let mut port = datacraft_core::port::MockPort::new();
    port.with_password("secret");
    let mut app = App::new(Box::new(port));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    assert!(app.password_prompt().is_some(), "prompt is raised");
    app.set_password_input("wrong");
    app.submit_password();
    assert!(
        app.password_prompt()
            .and_then(|prompt| prompt.error)
            .is_some(),
        "the wrong password keeps the prompt open"
    );
    app.set_password_input("secret");
    app.submit_password();
    assert!(app.password_prompt().is_none(), "the prompt closes");
    assert!(app.snapshot().database.is_some());
}

/// M29 regression: a row-mutating event must invalidate the per-tab row
/// cache — otherwise the datasheet keeps showing pre-undo rows.
#[test]
fn undo_refreshes_the_datasheet_row_cache() {
    use datacraft_core::ObjectId;
    use datacraft_core::ObjectKind;
    use datacraft_core::port::MockPort;
    use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};

    let notes = TableDef::new(
        "Notes",
        vec![FieldDef::new("body", FieldKind::Text { max_len: 40 })],
    );
    let mut app = App::new(Box::new(MockPort::with_tables(vec![notes])));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "notes.mdb".into(),
        password: None,
    });
    app.dispatch(datacraft_core::command::Command::OpenObject(ObjectId::new(
        ObjectKind::Table,
        "Notes",
    )));
    let tab = app.snapshot().tabs[0].id;
    assert_eq!(app.rows_for(tab).map(Vec::len), Some(0));

    app.dispatch(datacraft_core::command::Command::InsertRow {
        table: "Notes".into(),
        values: vec![FieldValue::Text("first".into())],
    });
    assert_eq!(
        app.rows_for(tab).map(Vec::len),
        Some(1),
        "the insert refreshes the cache"
    );

    app.dispatch(datacraft_core::command::Command::Undo);
    assert_eq!(
        app.rows_for(tab).map(Vec::len),
        Some(0),
        "undo must refresh the datasheet cache"
    );
}

/// Step 39.4 (item M31): a failing query run stores the reason for the tab
/// instead of leaving the results cache silently empty ("no runnable
/// result yet" forever).
#[test]
fn a_failing_query_run_stores_the_reason_for_the_tab() {
    use datacraft_core::qbe::{QbeRow, QbeValue};
    use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
    let definition = TableDef::new(
        "Customers",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    );
    let mut port = datacraft_core::port::MockPort::with_tables(vec![definition]);
    port.with_read_failure("disk on fire");
    let mut app = App::new(Box::new(port));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
    });
    app.dispatch(datacraft_core::command::Command::CreateQuery { name: "All".into() });
    app.dispatch(datacraft_core::command::Command::OpenObject(
        datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Query, "All"),
    ));
    app.dispatch(datacraft_core::command::Command::SetQbeCell {
        query: "All".into(),
        row: QbeRow::Table,
        col: 0,
        value: QbeValue::Text("Customers".into()),
    });
    app.dispatch(datacraft_core::command::Command::SetQbeCell {
        query: "All".into(),
        row: QbeRow::Field,
        col: 0,
        value: QbeValue::Text("id".into()),
    });
    let tab = app
        .snapshot()
        .tabs
        .iter()
        .find(|tab| tab.object_kind == "query")
        .expect("the query tab is open")
        .id;
    let reason = app
        .load_error(tab)
        .expect("the failing query run is stored for the tab");
    assert!(reason.contains("could not be run"), "{reason}");
}
