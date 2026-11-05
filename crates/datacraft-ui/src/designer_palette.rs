//! Designer palettes (step 11.6): record-source selector, source field
//! list and the Access toolbox, shared by the form and report design
//! views. Pure paint — every intent becomes one app flow call (which
//! dispatches exactly one command).

use eframe::egui;

use datacraft_core::ObjectKind;
use datacraft_core::palette::TOOLBOX;

use crate::app::App;
use crate::i18n::t;
use crate::labels::field_kind_label;

/// One field-list row: the snapshot keeps the kind locale-neutral and the
/// UI derives the display label (step 18.2, ADR-4).
struct PaletteField {
    name: String,
    kind: datacraft_core::schema::FieldKind,
    bindable: bool,
}

/// Renders the palette of one design tab.
pub fn designer_palette_ui(app: &mut App, kind: ObjectKind, object: &str, ui: &mut egui::Ui) {
    ui.set_min_width(200.0);
    ui.set_max_width(240.0);

    let (source, tables, fields) = palette_data(app, kind, object);

    let source_label = ui.label(egui::RichText::new(t("Record Source")).strong());
    let mut chosen: Option<String> = None;
    let source_combo = egui::ComboBox::from_id_salt(format!(
        "designer.{}.{}.source",
        kind.slug(),
        object.to_lowercase()
    ))
    .selected_text(if source.is_empty() {
        t("<none>").to_owned()
    } else {
        source.clone()
    })
    .show_ui(ui, |ui| {
        for table in &tables {
            if ui
                .selectable_label(source.eq_ignore_ascii_case(table), table)
                .clicked()
            {
                chosen = Some(table.clone());
            }
        }
    });
    // Name the combo for screen readers and the kittest's accessibility
    // check (the strong label above is decorative without this link).
    source_combo.response.labelled_by(source_label.id);
    if let Some(table) = chosen {
        app.set_record_source(kind, object, &table);
        return;
    }

    ui.add_space(4.0);
    ui.label(egui::RichText::new(t("Add Existing Fields")).strong());
    if source.is_empty() {
        ui.weak(t("Choose a record source first."));
    } else {
        let mut added: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt(format!(
                "designer.{}.{}.fields",
                kind.slug(),
                object.to_lowercase()
            ))
            .max_height(220.0)
            .show(ui, |ui| {
                for entry in &fields {
                    let label = format!("{}  ({})", entry.name, field_kind_label(&entry.kind));
                    let clicked = ui
                        .push_id(
                            format!("designer.field.{}", entry.name.to_lowercase()),
                            |ui| {
                                ui.add_enabled(entry.bindable, egui::Button::new(label).small())
                                    .clicked()
                            },
                        )
                        .inner;
                    if clicked {
                        added = Some(entry.name.clone());
                    }
                }
            });
        if let Some(field) = added {
            app.add_designer_field(kind, object, &field);
            return;
        }
    }

    ui.separator();
    ui.label(egui::RichText::new(t("Toolbox")).strong());
    let mut toolbox: Option<&'static str> = None;
    for entry in TOOLBOX {
        let clicked = ui
            .push_id(format!("designer.toolbox.{}", entry.id), |ui| {
                ui.small_button(t(entry.label)).clicked()
            })
            .inner;
        if clicked {
            toolbox = Some(entry.id);
        }
    }
    if let Some(entry_id) = toolbox {
        app.add_toolbox_control(kind, object, entry_id);
    }
}

/// Record source, table names and the source's field list of one design
/// tab (owned copies: the paint code never holds the snapshot borrow).
fn palette_data(
    app: &App,
    kind: ObjectKind,
    object: &str,
) -> (String, Vec<String>, Vec<PaletteField>) {
    let Some(db) = app.snapshot().database.as_ref() else {
        return (String::new(), Vec::new(), Vec::new());
    };
    let tables: Vec<String> = db.tables.iter().map(|t| t.name.clone()).collect();
    let source = match kind {
        ObjectKind::Form => db
            .forms
            .iter()
            .find(|form| form.name.eq_ignore_ascii_case(object))
            .map(|form| form.source.clone()),
        ObjectKind::Report => db
            .reports
            .iter()
            .find(|report| report.name.eq_ignore_ascii_case(object))
            .map(|report| report.source.clone()),
        ObjectKind::Table | ObjectKind::Query | ObjectKind::Macro | ObjectKind::Module => None,
    }
    .unwrap_or_default();
    let fields = db
        .tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(&source))
        .map(|table| {
            table
                .fields
                .iter()
                .map(|field| PaletteField {
                    name: field.name.clone(),
                    kind: field.kind,
                    bindable: datacraft_core::palette::field_control_size(&field.kind).is_some(),
                })
                .collect()
        })
        .unwrap_or_default();
    (source, tables, fields)
}
