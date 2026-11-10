//! Field-property rendering in the design view (step 32.1, item H45).
//!
//! The sidecar overlay lands in the snapshot; the design grid and the
//! property sheet must render the caption, the required state and the
//! index mode without any UI-side property logic.

use datacraft_core::ObjectId;
use datacraft_core::ObjectKind;
use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{
    FieldDef, FieldKind, FieldProperties, IndexMode, NumericKind, TableDef,
};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn table() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef {
                name: "id".into(),
                kind: FieldKind::Number(NumericKind::LongInteger),
                properties: FieldProperties {
                    caption: Some("Identifier".into()),
                    required: true,
                    indexed: IndexMode::NoDuplicates,
                    ..FieldProperties::default()
                },
            },
            FieldDef {
                name: "name".into(),
                kind: FieldKind::Text { max_len: 40 },
                properties: FieldProperties {
                    caption: Some("Customer name".into()),
                    validation_text: Some("A name is required".into()),
                    ..FieldProperties::default()
                },
            },
        ],
    )
}

#[test]
fn the_design_view_renders_sidecar_properties() {
    let mut app = App::new(Box::new(MockPort::with_tables(vec![table()])));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObjectInMode {
        object: ObjectId::new(ObjectKind::Table, "Customers"),
        mode: datacraft_core::state::TabMode::Design,
    });
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            let Some(tab) = app.snapshot().tabs.first().cloned() else {
                return;
            };
            datacraft_ui::design::design_ui(app, &tab, ui);
        },
        app,
    );
    harness.run();
    for label in ["Customer name", "Identifier", "Yes (No Duplicates)"] {
        let node = harness
            .get_all_by_label(label)
            .next()
            .unwrap_or_else(|| panic!("the property {label:?} renders in the design view"));
        assert!(
            node.rect().width() > 0.0,
            "the property {label:?} renders in the design view"
        );
    }
}
