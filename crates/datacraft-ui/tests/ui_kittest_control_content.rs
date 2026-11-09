//! The designer control-content panel (step 63.2, item H56): a placed
//! control's one content property is edited beside the palette and the
//! runtime/preview renders it. Both designers are pinned — set and render.

use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_core::state::TabMode;
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers_app() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![
            FieldValue::Text("alpha".to_owned()),
            FieldValue::Boolean(true),
        ]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
    });
    app
}

fn label(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label {
            text: "default".into(),
        },
        rect: datacraft_core::canvas::Rect {
            x: 8.0,
            y: 8.0,
            width: 120.0,
            height: 24.0,
        },
    }
}

type Render = fn(&mut App, &datacraft_core::snapshot::TabSnapshot, &mut eframe::egui::Ui);

/// Renders the designer and the runtime/preview stacked from the same
/// state — the apply lands in the snapshot, the second surface proves it.
fn harness(app: App, design: Render, output: Render) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            design(app, &tab, ui);
            ui.separator();
            output(app, &tab, ui);
        },
        app,
    );
    harness.run();
    harness
}

fn tree_dump(harness: &Harness<'_, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        lines.push(format!("{:?}|{label}|{value}", node.role()));
    }
    lines.join("\n")
}

fn form_label_text(app: &App) -> String {
    let db = app.snapshot().database.clone().expect("open");
    let form = db.forms.iter().find(|f| f.name == "Card").expect("form");
    match &form
        .controls
        .iter()
        .find(|c| c.name == "C1")
        .expect("control")
        .kind
    {
        ControlKind::Label { text } => text.clone(),
        other => panic!("expected a label, got {other:?}"),
    }
}

/// The form designer's panel sets the label text, and the runtime renders
/// it — the reported H56 gap closed.
#[test]
fn the_form_designer_sets_a_label_text_and_the_runtime_renders_it() {
    let mut app = customers_app();
    app.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    app.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    app.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: label("C1"),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Form, "Card"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_form_selection(tab, "C1".into());

    let mut harness = harness(
        app,
        datacraft_ui::form_design::form_design_ui,
        datacraft_ui::form_runtime::form_runtime_ui,
    );
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Control C1"),
        "the panel names the selected control:\n{dump}"
    );

    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Text")
        .set_text_value_accesskit("Hello world");
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();

    assert_eq!(form_label_text(harness.state()), "Hello world");
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Hello world"),
        "the form runtime renders the edited text:\n{dump}"
    );
}

/// The report designer's panel sets a label text and the print preview
/// renders it.
#[test]
fn the_report_designer_sets_a_label_text_and_the_preview_renders_it() {
    let mut app = customers_app();
    app.dispatch(Command::CreateReport {
        name: "Sheet".into(),
    });
    app.dispatch(Command::SetReportSource {
        report: "Sheet".into(),
        table: "Customers".into(),
    });
    app.dispatch(Command::AddReportControl {
        report: "Sheet".into(),
        control: label("Title"),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Report, "Sheet"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_report_selection(tab, "Title".into());

    let mut harness = harness(
        app,
        datacraft_ui::report_design::report_design_ui,
        datacraft_ui::report_design::report_preview_ui,
    );
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Text")
        .set_text_value_accesskit("Quarterly report");
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();

    let db = harness.state().snapshot().database.clone().expect("open");
    let report = db
        .reports
        .iter()
        .find(|r| r.name == "Sheet")
        .expect("report");
    let text = match &report
        .controls
        .iter()
        .find(|c| c.name == "Title")
        .expect("control")
        .kind
    {
        ControlKind::Label { text } => text.clone(),
        other => panic!("expected a label, got {other:?}"),
    };
    assert_eq!(text, "Quarterly report");
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Quarterly report"),
        "the print preview renders the edited text:\n{dump}"
    );
}

fn combo(name: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::ComboBox {
            bound_field: Some("name".into()),
            row_source: String::new(),
            display_field: String::new(),
        },
        rect: datacraft_core::canvas::Rect {
            x: 8.0,
            y: 8.0,
            width: 160.0,
            height: 24.0,
        },
    }
}

fn combo_parts(app: &App) -> (String, String) {
    let db = app.snapshot().database.clone().expect("open");
    let form = db.forms.iter().find(|f| f.name == "Card").expect("form");
    match &form
        .controls
        .iter()
        .find(|c| c.name == "Pick")
        .expect("control")
        .kind
    {
        ControlKind::ComboBox {
            row_source,
            display_field,
            ..
        } => (row_source.clone(), display_field.clone()),
        other => panic!("expected a combo box, got {other:?}"),
    }
}

/// The designer sets the combo's row source and display field; the
/// runtime lists the source rows. An unknown display field is rejected
/// precisely and leaves the stored source untouched.
#[test]
fn the_form_designer_sets_a_combo_row_source_and_the_runtime_lists_it() {
    let mut app = customers_app();
    app.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    app.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    app.dispatch(Command::AddFormControl {
        form: "Card".into(),
        control: combo("Pick"),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: datacraft_core::ObjectId::new(datacraft_core::ObjectKind::Form, "Card"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_form_selection(tab, "Pick".into());

    let mut harness = harness(
        app,
        datacraft_ui::form_design::form_design_ui,
        datacraft_ui::form_runtime::form_runtime_ui,
    );
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Row source")
        .set_text_value_accesskit("Customers");
    harness.run();
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Display field")
        .set_text_value_accesskit("name");
    harness.run();
    {
        let mut applies = harness.get_all_by_label("Apply");
        applies.nth(1).expect("the combo panel's Apply").click();
    }
    harness.run();

    assert_eq!(
        combo_parts(harness.state()),
        ("Customers".to_owned(), "name".to_owned())
    );
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::Button, "Pick values")
        .click();
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("alpha"),
        "the runtime list shows the source rows:\n{dump}"
    );

    // An unknown display field is rejected; the stored source survives.
    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Display field")
        .set_text_value_accesskit("missing");
    harness.run();
    {
        let mut applies = harness.get_all_by_label("Apply");
        applies.nth(1).expect("the combo panel's Apply").click();
    }
    harness.run();
    assert_eq!(
        combo_parts(harness.state()),
        ("Customers".to_owned(), "name".to_owned()),
        "the rejected apply leaves the stored row source"
    );
    assert!(
        harness
            .state()
            .last_error()
            .is_some_and(|error| error.contains("missing")),
        "the rejection names the unknown field"
    );
}
