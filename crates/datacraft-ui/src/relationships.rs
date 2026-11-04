//! Relationship mapper pane (step 7.1, item H25).
//!
//! A floating window listing the snapshot's relationships and painting
//! the table boxes with connection lines. Pure paint: the data comes from
//! the snapshot, and every intent dispatches exactly one command — a
//! per-row delete and the create form's single `CreateRelationship`
//! (step 41.1, item M45). Both write affordances render disabled with
//! the localized reason when the driver reports less than `Support::Yes`,
//! mirroring the dispatch gate.

use eframe::egui;

use datacraft_core::availability::ids;
use datacraft_core::command::Command;
use datacraft_core::port::Relationship;
use datacraft_core::snapshot::{FieldSnapshot, TableEntrySnapshot};

use crate::app::App;
use crate::availability::restricted;
use crate::i18n::{plural, t};

/// Box geometry for the canvas.
const BOX_SIZE: egui::Vec2 = egui::Vec2::new(150.0, 64.0);
const BOX_GAP: f32 = 28.0;
const PER_ROW: usize = 3;

/// Picker labels (i18n keys). They flow through variables, so the i18n
/// parity test picks them up from these constants.
pub(crate) const LABEL_REFERENCING_TABLE: &str = "Referencing table";
pub(crate) const LABEL_REFERENCING_FIELD: &str = "Referencing field";
pub(crate) const LABEL_REFERENCED_TABLE: &str = "Referenced table";
pub(crate) const LABEL_REFERENCED_FIELD: &str = "Referenced field";

/// Draft selections of the relationship-create form (presentation state;
/// the command itself is built on dispatch, step 41.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelationshipDraft {
    /// Referencing (child) table.
    pub table: String,
    /// Referencing (child) column.
    pub column: String,
    /// Referenced (parent) table.
    pub related_table: String,
    /// Referenced (parent) column.
    pub related_column: String,
}

/// Renders the floating relationship mapper (when open).
pub fn mapper_window(app: &mut App, ui: &mut egui::Ui) {
    if !app.snapshot().relationship_mapper_open {
        return;
    }
    let snapshot = app.snapshot().clone();
    let Some(db) = snapshot.database.clone() else {
        return;
    };
    let create_reason = restricted(&snapshot, ids::RELATIONSHIPS_CREATE).copied();
    let delete_reason = restricted(&snapshot, ids::RELATIONSHIPS_DELETE).copied();
    let mut close = false;
    let mut delete: Option<Relationship> = None;
    let mut create: Option<Relationship> = None;
    egui::Window::new(t("Relationships"))
        .id(egui::Id::unique("datacraft.relationships"))
        .default_width(640.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(plural(
                    db.relationships.len(),
                    "{count} relationship",
                    "{count} relationships",
                ));
                // Create affordance (step 41.1): one command per flow,
                // capability-disabled for the open source.
                let new_response = match create_reason {
                    Some(reason) => ui
                        .add_enabled(false, egui::Button::new(t("New Relationship")))
                        .on_hover_text(crate::availability::reason_text(&reason)),
                    None => ui.button(t("New Relationship")),
                };
                if new_response.clicked() {
                    app.relationship_draft = Some(draft_from(&db.tables));
                }
                if ui.small_button(t("Close")).clicked() {
                    close = true;
                }
            });
            ui.separator();

            // Textual list with one delete affordance per row (step 41.1).
            for rel in &db.relationships {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{}.{}  →  {}.{}",
                        rel.table, rel.column, rel.related_table, rel.related_column
                    ));
                    let delete_response = match delete_reason {
                        Some(reason) => ui
                            .add_enabled(false, egui::Button::new(t("Delete")).small())
                            .on_hover_text(crate::availability::reason_text(&reason)),
                        None => ui.small_button(t("Delete")),
                    };
                    if delete_response.clicked() {
                        delete = Some(rel.clone());
                    }
                });
            }
            if db.relationships.is_empty() {
                ui.weak(t("This database has no relationships."));
            }
            ui.separator();

            if app.relationship_draft.is_some() {
                create_form(app, &db.tables, ui, &mut create);
                ui.separator();
            }

            // Canvas: deterministic box layout, lines between related tables.
            let tables: Vec<&String> = {
                let mut names: Vec<&String> = db
                    .relationships
                    .iter()
                    .flat_map(|r| [&r.table, &r.related_table])
                    .collect();
                names.sort_by_key(|a| a.to_lowercase());
                names.dedup();
                names
            };
            let cols = PER_ROW;
            let rows = tables.len().div_ceil(cols).max(1);
            let (canvas_rect, response) = ui.allocate_exact_size(
                egui::vec2(
                    cols as f32 * (BOX_SIZE.x + BOX_GAP),
                    rows as f32 * (BOX_SIZE.y + BOX_GAP),
                ),
                egui::Sense::hover(),
            );
            let box_rect = |i: usize| -> egui::Rect {
                let col = i % cols;
                let row = i / cols;
                egui::Rect::from_min_size(
                    canvas_rect.min
                        + egui::vec2(
                            col as f32 * (BOX_SIZE.x + BOX_GAP),
                            row as f32 * (BOX_SIZE.y + BOX_GAP),
                        ),
                    BOX_SIZE,
                )
            };
            let index_of = |name: &String| tables.iter().position(|t| t.eq_ignore_ascii_case(name));
            let stroke = ui.visuals().widgets.noninteractive.fg_stroke;

            // Lines first (under the boxes).
            for rel in &db.relationships {
                let (Some(a), Some(b)) = (index_of(&rel.table), index_of(&rel.related_table))
                else {
                    continue;
                };
                let (ra, rb) = (box_rect(a), box_rect(b));
                ui.painter()
                    .line_segment([ra.center(), rb.center()], stroke);
            }
            for (i, name) in tables.iter().enumerate() {
                let rect = box_rect(i);
                ui.painter()
                    .rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                ui.painter()
                    .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
                ui.painter().text(
                    rect.left_top() + egui::vec2(6.0, 6.0),
                    egui::Align2::LEFT_TOP,
                    name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    stroke.color,
                );
            }
            let _ = response;
        });
    if close {
        app.relationship_draft = None;
        app.dispatch(Command::ToggleRelationshipMapper);
    }
    if let Some(relationship) = delete {
        app.dispatch(Command::DeleteRelationship { relationship });
    }
    if let Some(relationship) = create {
        let events = app.dispatch(Command::CreateRelationship { relationship });
        // A failed create keeps the draft so the user can correct it; the
        // precise reason surfaces through the status bar.
        if !events
            .iter()
            .any(|event| matches!(event, datacraft_core::event::Event::Failed { .. }))
        {
            app.relationship_draft = None;
        }
    }
}

/// Renders the create form: four pickers over the snapshot's schema and
/// the single dispatch button (step 41.1).
fn create_form(
    app: &mut App,
    tables: &[TableEntrySnapshot],
    ui: &mut egui::Ui,
    create: &mut Option<Relationship>,
) {
    let Some(draft) = app.relationship_draft.as_mut() else {
        return;
    };
    pick_table(ui, LABEL_REFERENCING_TABLE, &mut draft.table, tables);
    reset_column(tables, &draft.table, &mut draft.column);
    pick_field(
        ui,
        LABEL_REFERENCING_FIELD,
        &mut draft.column,
        tables,
        &draft.table,
    );
    pick_table(ui, LABEL_REFERENCED_TABLE, &mut draft.related_table, tables);
    reset_column(tables, &draft.related_table, &mut draft.related_column);
    pick_field(
        ui,
        LABEL_REFERENCED_FIELD,
        &mut draft.related_column,
        tables,
        &draft.related_table,
    );

    let ready = !draft.table.is_empty()
        && !draft.column.is_empty()
        && !draft.related_table.is_empty()
        && !draft.related_column.is_empty();
    if ready && ui.button(t("Create Relationship")).clicked() {
        *create = Some(Relationship {
            table: draft.table.clone(),
            column: draft.column.clone(),
            related_table: draft.related_table.clone(),
            related_column: draft.related_column.clone(),
        });
    }
}

/// Table picker. `from_label` names the combo for screen readers (and
/// the kittest's accessibility check); the selection rides the value.
fn pick_table(
    ui: &mut egui::Ui,
    label: &'static str,
    selected: &mut String,
    tables: &[TableEntrySnapshot],
) {
    let mut chosen = selected.clone();
    egui::ComboBox::from_label(t(label))
        .selected_text(selected_text(selected))
        .show_ui(ui, |ui| {
            for table in tables {
                if ui
                    .selectable_label(chosen.eq_ignore_ascii_case(&table.name), &table.name)
                    .clicked()
                {
                    chosen = table.name.clone();
                }
            }
        });
    if !chosen.eq_ignore_ascii_case(selected) {
        *selected = chosen;
    }
}

/// Field picker over the chosen table's fields.
fn pick_field(
    ui: &mut egui::Ui,
    label: &'static str,
    selected: &mut String,
    tables: &[TableEntrySnapshot],
    table: &str,
) {
    let fields: &[FieldSnapshot] = tables
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
        .map_or(&[], |candidate| &candidate.fields);
    let mut chosen = selected.clone();
    egui::ComboBox::from_label(t(label))
        .selected_text(selected_text(selected))
        .show_ui(ui, |ui| {
            for field in fields {
                if ui
                    .selectable_label(chosen.eq_ignore_ascii_case(&field.name), &field.name)
                    .clicked()
                {
                    chosen = field.name.clone();
                }
            }
        });
    if !chosen.eq_ignore_ascii_case(selected) {
        *selected = chosen;
    }
}

/// Displayed picker selection; `<none>` until a choice is made.
fn selected_text(selected: &str) -> String {
    if selected.is_empty() {
        t("<none>").to_owned()
    } else {
        selected.to_owned()
    }
}

/// Re-seats the column when its table changed (a picked column must
/// belong to the picked table).
fn reset_column(tables: &[TableEntrySnapshot], table: &str, column: &mut String) {
    let fields: &[FieldSnapshot] = tables
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(table))
        .map_or(&[], |candidate| &candidate.fields);
    if !fields
        .iter()
        .any(|field| field.name.eq_ignore_ascii_case(column))
    {
        *column = fields
            .first()
            .map_or_else(String::new, |field| field.name.clone());
    }
}

/// Prefills the draft with the schema's first tables/columns (Access
/// opens its relationship dialog pre-selected as well).
fn draft_from(tables: &[TableEntrySnapshot]) -> RelationshipDraft {
    let first = tables.first();
    let second = tables.get(1).or(first);
    let name =
        |table: Option<&TableEntrySnapshot>| table.map_or_else(String::new, |t| t.name.clone());
    let field = |table: Option<&TableEntrySnapshot>| {
        table
            .and_then(|t| t.fields.first())
            .map_or_else(String::new, |f| f.name.clone())
    };
    RelationshipDraft {
        table: name(first),
        column: field(first),
        related_table: name(second),
        related_column: field(second),
    }
}
