//! Navigation pane rendering (step 5.3, item H19; Access fidelity 12.3).
//!
//! Fully snapshot-driven: objects come from `UiStateSnapshot` with their
//! convention test ids (`navpane.<expanded|collapsed>.node.<kind>.<name>`)
//! used as egui id salts; a click dispatches `OpenObject`. Sections render
//! as collapsible groups in the Access order (Tables, Queries, Forms,
//! Reports). No hidden state.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;

use crate::app::App;
use crate::i18n::{t, tr};

/// Section headers (English keys; also the id-salt prefix, so automation
/// ids stay language-neutral). The catalog parity test enumerates them.
pub(crate) const SECTION_TABLES: &str = "Tables";
pub(crate) const SECTION_QUERIES: &str = "Queries";
pub(crate) const SECTION_FORMS: &str = "Forms";
pub(crate) const SECTION_REPORTS: &str = "Reports";
pub(crate) const SECTION_MACROS: &str = "Macros";
pub(crate) const SECTION_MODULES: &str = "Modules";

/// Renders the navigation pane contents (the panel itself is shown by the
/// shell only while `nav_pane_visible`).
pub fn nav_pane_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new(t("All Access Objects")).strong());

    let snapshot = app.snapshot();
    let Some(db) = &snapshot.database else {
        ui.label(t("No database open"));
        return;
    };
    if db.tables.is_empty()
        && db.forms.is_empty()
        && db.reports.is_empty()
        && db.queries.is_empty()
        && db.macros.is_empty()
        && db
            .vba
            .as_ref()
            .is_none_or(|project| project.modules.is_empty())
    {
        ui.label(t("No objects"));
        return;
    }
    let active_object = snapshot
        .active_tab
        .and_then(|active| active_object_name(&snapshot.tabs, active));

    let mut intent: Option<(datacraft_core::ObjectKind, String)> = None;
    let mut delete_intent: Option<(datacraft_core::ObjectKind, String)> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        // Section: a collapsible group whose entries open on click. The
        // header carries the Access object-group names.
        let mut section =
            |header: &'static str, entries: Vec<(String, datacraft_core::ObjectKind)>| {
                if entries.is_empty() {
                    return;
                }
                egui::CollapsingHeader::new(egui::RichText::new(t(header)).strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        for (name, kind) in entries {
                            // The id salt keeps the English header so automation
                            // ids stay language-neutral.
                            ui.push_id(format!("nav.{header}.{name}"), |ui| {
                                let is_active = active_object
                                    .as_deref()
                                    .is_some_and(|active| active.eq_ignore_ascii_case(&name));
                                ui.horizontal(|ui| {
                                    crate::icons::icon(ui, kind.slug(), is_active);
                                    if ui.selectable_label(is_active, &name).clicked() {
                                        intent = Some((kind, name.clone()));
                                    }
                                    if deletable(kind)
                                        && ui
                                            .small_button("✕")
                                            .on_hover_text(tr("Delete {name}", &[("name", &name)]))
                                            .clicked()
                                    {
                                        delete_intent = Some((kind, name));
                                    }
                                });
                            });
                        }
                    });
            };

        section(
            SECTION_TABLES,
            db.tables
                .iter()
                .map(|t| (t.name.clone(), datacraft_core::ObjectKind::Table))
                .collect(),
        );
        section(
            SECTION_QUERIES,
            db.queries
                .iter()
                .map(|q| (q.name.clone(), datacraft_core::ObjectKind::Query))
                .collect(),
        );
        section(
            SECTION_FORMS,
            db.forms
                .iter()
                .map(|f| (f.name.clone(), datacraft_core::ObjectKind::Form))
                .collect(),
        );
        section(
            SECTION_REPORTS,
            db.reports
                .iter()
                .map(|r| (r.name.clone(), datacraft_core::ObjectKind::Report))
                .collect(),
        );
        section(
            SECTION_MACROS,
            db.macros
                .iter()
                .map(|m| (m.name.clone(), datacraft_core::ObjectKind::Macro))
                .collect(),
        );
        section(
            SECTION_MODULES,
            db.vba
                .as_ref()
                .map(|project| {
                    project
                        .modules
                        .iter()
                        .map(|m| (m.name.clone(), datacraft_core::ObjectKind::Module))
                        .collect()
                })
                .unwrap_or_default(),
        );
    });
    if let Some((kind, name)) = delete_intent {
        app.delete_object(kind, &name);
    } else if let Some((kind, name)) = intent {
        app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            kind, name,
        )));
    }
}

fn active_object_name(
    tabs: &[datacraft_core::snapshot::TabSnapshot],
    active: TabId,
) -> Option<String> {
    tabs.iter()
        .find(|tab| tab.id == active)
        .map(|tab| tab.object_name.clone())
}

/// Macros and modules are a read-only inventory (steps 17.1/17.2): no
/// delete affordance.
fn deletable(kind: datacraft_core::ObjectKind) -> bool {
    !matches!(
        kind,
        datacraft_core::ObjectKind::Macro | datacraft_core::ObjectKind::Module
    )
}
