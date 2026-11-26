//! The designer property sheet (phase 102, item M133): a placed control's
//! name and presentation properties are edited beside the palette, each
//! edit dispatches one command and the effect is visible — the renamed
//! control stays selected, a hidden control disappears from the print
//! preview.
//!
//! Core validation and persistence are pinned by
//! `datacraft-core/tests/control_properties.rs`; this suite pins the two
//! designer surfaces.

use datacraft_core::command::Command;
use datacraft_core::form::{ControlDef, ControlKind, ControlProperties, TextAlign};
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, TableDef};
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers_app() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Customers",
        vec![FieldDef::new("name", FieldKind::Text { max_len: 50 })],
    )]);
    port.with_row_data(
        "Customers",
        vec![vec![FieldValue::Text("alpha".to_owned())]],
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn label(name: &str, text: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        kind: ControlKind::Label { text: text.into() },
        rect: datacraft_core::canvas::Rect {
            x: 8.0,
            y: 8.0,
            width: 160.0,
            height: 24.0,
        },
        properties: ControlProperties::default(),
    }
}

type Render = fn(&mut App, &datacraft_core::snapshot::TabSnapshot, &mut eframe::egui::Ui);

fn harness(app: App, design: Render, output: Render) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            ui.set_max_width(1400.0);
            design(app, &tab, ui);
            ui.separator();
            output(app, &tab, ui);
        },
        app,
    );
    harness.set_size(eframe::egui::vec2(1400.0, 1000.0));
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

/// The sheet's name edit commits through its own Rename button (the
/// content panel's Apply stays unambiguous next to it).
fn click_sheet_apply(harness: &mut Harness<'_, App>) {
    harness.get_by_label("Rename").click();
    harness.run();
}

fn form_control(app: &App) -> ControlDef {
    let db = app.snapshot().database.clone().expect("open");
    db.forms
        .iter()
        .find(|f| f.name == "Card")
        .expect("form")
        .controls
        .first()
        .expect("control")
        .clone()
}

fn form_designer(name: &str, text: &str) -> App {
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
        control: label(name, text),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Form, "Card"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_form_selection(tab, name.into());
    app
}

/// The form designer's sheet renames the control (the event's canonical
/// name keeps it selected) and applies every style property.
#[test]
fn the_form_sheet_renames_and_styles_the_control() {
    let app = form_designer("C1", "Customers");
    let mut harness = harness(
        app,
        datacraft_ui::form_design::form_design_ui,
        datacraft_ui::form_runtime::form_runtime_ui,
    );
    assert!(
        tree_dump(&harness).contains("Label||Properties"),
        "the sheet renders for the selected control:\n{}",
        tree_dump(&harness)
    );

    harness
        .get_by_role_and_label(eframe::egui::accesskit::Role::TextInput, "Name")
        .set_text_value_accesskit("Heading");
    harness.run();
    click_sheet_apply(&mut harness);
    let control = form_control(harness.state());
    assert_eq!(control.name, "Heading", "the rename landed");
    let tab = harness.state().snapshot().tabs[0].id;
    assert_eq!(
        harness.state().form_selection(tab).as_deref(),
        Some("Heading"),
        "the renamed control stays selected"
    );

    harness.get_by_label("Bold").click();
    harness.run();
    assert!(form_control(harness.state()).properties.bold);
    // The runtime renders the label in the SemiBold face (no tofu, no
    // panic under the kittest font policy).
    harness.get_by_label("Alignment").click();
    harness.run();
    harness.get_by_label("Center").click();
    harness.run();
    assert_eq!(
        form_control(harness.state()).properties.align,
        TextAlign::Center
    );

    // The runtime renders the label widget; hiding it removes the node.
    assert!(
        tree_dump(&harness).contains("Label||Customers"),
        "the runtime renders the visible label:\n{}",
        tree_dump(&harness)
    );
    harness.get_by_label("Visible").click();
    harness.run();
    assert!(!form_control(harness.state()).properties.visible);
    let dump = tree_dump(&harness);
    assert!(
        !dump.contains("Label||Customers"),
        "the runtime no longer renders the hidden control:\n{dump}"
    );
}

/// The report sheet hides a control from the print preview — the property
/// is not only stored, it changes what prints.
#[test]
fn the_report_sheet_hides_the_control_from_the_preview() {
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
        control: label("Secret", "Internal"),
    });
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Report, "Sheet"),
        mode: TabMode::Design,
    });
    let tab = app.snapshot().tabs[0].id;
    app.set_report_selection(tab, "Secret".into());

    let mut harness = harness(
        app,
        datacraft_ui::report_design::report_design_ui,
        datacraft_ui::report_design::report_preview_ui,
    );
    // The UI's preview cache is exactly what the preview painter draws.
    let preview_texts = |harness: &Harness<'_, App>| -> Vec<String> {
        let tab = harness.state().snapshot().tabs[0].id;
        harness
            .state()
            .preview_pages(tab)
            .iter()
            .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
            .collect()
    };
    assert_eq!(preview_texts(&harness), vec!["Internal".to_owned()]);
    harness.get_by_label("Visible").click();
    harness.run();
    assert!(
        !harness
            .state()
            .snapshot()
            .database
            .clone()
            .expect("open")
            .reports[0]
            .controls[0]
            .properties
            .visible
    );
    assert!(
        preview_texts(&harness).is_empty(),
        "the hidden control leaves the print preview: {:?}",
        preview_texts(&harness)
    );
}
