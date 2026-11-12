//! Navigation pane rendering (step 5.3, item H19; Access fidelity 12.3;
//! pane UX in step 21.5, item U9).
//!
//! Fully snapshot-driven: objects come from `UiStateSnapshot` with their
//! convention test ids (`navpane.<expanded|collapsed>.node.<kind>.<name>`)
//! used as egui id salts. The pane header is the Access category dropdown
//! (single entry today), objects are opened/designed/deleted from their
//! context menu, and each group header carries the name/type sort menu —
//! the ordering itself is the pure core function `nav::sort_entries`. No
//! hidden state.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::nav::NavSortOrder;
use datacraft_core::state::TabMode;
use datacraft_core::{ObjectId, ObjectKind};

use crate::app::App;
use crate::i18n::{t, tr};

/// Section headers (English keys; also the id-salt prefix, so automation
/// ids stay language-neutral). The catalog parity test enumerates them.
pub(crate) const SECTION_TABLES: &str = "Tables";
pub(crate) const SECTION_QUERIES: &str = "Queries";
pub(crate) const SECTION_VIEWS: &str = "Views";
pub(crate) const SECTION_FORMS: &str = "Forms";
pub(crate) const SECTION_REPORTS: &str = "Reports";
pub(crate) const SECTION_MACROS: &str = "Macros";
pub(crate) const SECTION_MODULES: &str = "Modules";

/// Renders the navigation pane contents (the panel itself is shown by the
/// shell only while `nav_pane_visible`).
pub fn nav_pane_ui(app: &mut App, ui: &mut egui::Ui) {
    // Access category dropdown: one entry until the category model grows.
    // A menu button (not a ComboBox) carries an accessible name.
    ui.push_id("navpane.category", |ui| {
        ui.menu_button(t("All Access Objects"), |ui| {
            let _ = ui.selectable_label(true, t("All Access Objects"));
        });
    });

    let snapshot = app.snapshot();
    let Some(db) = &snapshot.database else {
        ui.label(t("No database open"));
        return;
    };
    if db.tables.is_empty()
        && db.forms.is_empty()
        && db.reports.is_empty()
        && db.queries.is_empty()
        && db.views.is_empty()
        && db.native_views.is_empty()
        && db.native_objects.is_empty()
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
    let sort = app.nav_sort;

    let mut open_intent: Option<(ObjectKind, String, Option<TabMode>)> = None;
    let mut rename_intent: Option<(ObjectKind, String)> = None;
    let mut delete_intent: Option<(ObjectKind, String)> = None;
    let mut sort_intent: Option<NavSortOrder> = None;
    let mut new_view = false;
    egui::ScrollArea::vertical().show(ui, |ui| {
        // Section: a collapsible group whose entries open from the context
        // menu (Open / Design / Delete); the header carries the sort menu.
        let mut section = |header: &'static str, mut entries: Vec<(String, ObjectKind)>| {
            if entries.is_empty() {
                return;
            }
            datacraft_core::nav::sort_entries(&mut entries, sort);
            let group = egui::CollapsingHeader::new(egui::RichText::new(t(header)).strong())
                .default_open(true)
                .show(ui, |ui| {
                    for (name, kind) in entries {
                        ui.push_id(format!("nav.{header}.{name}"), |ui| {
                            let is_active = active_object
                                .as_deref()
                                .is_some_and(|active| active.eq_ignore_ascii_case(&name));
                            ui.horizontal(|ui| {
                                crate::icons::icon(ui, kind.slug(), is_active);
                                let entry = ui.selectable_label(is_active, &name);
                                if entry.clicked() {
                                    open_intent = Some((kind, name.clone(), None));
                                }
                                entry.context_menu(|ui| {
                                    let open = ui
                                        .push_id(format!("nav.{header}.{name}.menu.open"), |ui| {
                                            ui.button(t("Open"))
                                        })
                                        .inner
                                        .clicked();
                                    if open {
                                        open_intent = Some((kind, name.clone(), None));
                                        ui.close();
                                    }
                                    if kind.has_design_view() {
                                        let design = ui
                                            .push_id(
                                                format!("nav.{header}.{name}.menu.design"),
                                                |ui| ui.button(t("Design")),
                                            )
                                            .inner
                                            .clicked();
                                        if design {
                                            open_intent =
                                                Some((kind, name.clone(), Some(TabMode::Design)));
                                            ui.close();
                                        }
                                    }
                                    if renamable(kind) {
                                        // Tables gate on the rename
                                        // capability (ADR-7); sidecar
                                        // objects are always renamable.
                                        let reason = if kind == ObjectKind::Table {
                                            crate::availability::restricted(
                                                app.snapshot(),
                                                datacraft_core::availability::ids::NAV_RENAME,
                                            )
                                        } else {
                                            None
                                        };
                                        let rename = ui
                                            .push_id(
                                                format!("nav.{header}.{name}.menu.rename"),
                                                |ui| match reason {
                                                    Some(reason) => ui
                                                        .add_enabled(
                                                            false,
                                                            egui::Button::new(t("Rename")),
                                                        )
                                                        .on_hover_text(
                                                            crate::availability::reason_text(
                                                                reason,
                                                            ),
                                                        ),
                                                    None => ui.button(t("Rename")),
                                                },
                                            )
                                            .inner
                                            .clicked();
                                        if rename {
                                            rename_intent = Some((kind, name.clone()));
                                            ui.close();
                                        }
                                    }
                                    if deletable(kind) {
                                        // Capability-disabled for the open
                                        // source (ADR-7): the entry renders
                                        // muted with the reason, no click.
                                        let reason = crate::availability::restricted(
                                            app.snapshot(),
                                            datacraft_core::availability::ids::NAV_DELETE,
                                        );
                                        let remove = ui
                                            .push_id(
                                                format!("nav.{header}.{name}.menu.delete"),
                                                |ui| match reason {
                                                    Some(reason) => ui
                                                        .add_enabled(
                                                            false,
                                                            egui::Button::new(tr(
                                                                "Delete {name}",
                                                                &[("name", &name)],
                                                            )),
                                                        )
                                                        .on_hover_text(
                                                            crate::availability::reason_text(
                                                                reason,
                                                            ),
                                                        ),
                                                    None => ui.button(tr(
                                                        "Delete {name}",
                                                        &[("name", &name)],
                                                    )),
                                                },
                                            )
                                            .inner
                                            .clicked();
                                        if remove {
                                            delete_intent = Some((kind, name.clone()));
                                            ui.close();
                                        }
                                    }
                                });
                            });
                        });
                    }
                });
            group.header_response.context_menu(|ui| {
                let by_name = ui
                    .push_id(format!("nav.{header}.menu.sort_name"), |ui| {
                        ui.button(t("Sort by Name"))
                    })
                    .inner
                    .clicked();
                if by_name {
                    sort_intent = Some(NavSortOrder::Name);
                    ui.close();
                }
                let by_type = ui
                    .push_id(format!("nav.{header}.menu.sort_type"), |ui| {
                        ui.button(t("Sort by Type"))
                    })
                    .inner
                    .clicked();
                if by_type {
                    sort_intent = Some(NavSortOrder::Type);
                    ui.close();
                }
            });
        };

        section(
            SECTION_TABLES,
            db.tables
                .iter()
                .map(|t| (t.name.clone(), ObjectKind::Table))
                .collect(),
        );
        section(
            SECTION_QUERIES,
            db.queries
                .iter()
                .map(|q| (q.name.clone(), ObjectKind::Query))
                .collect(),
        );
        section(
            SECTION_VIEWS,
            db.views
                .iter()
                .map(|v| (v.name.clone(), ObjectKind::View))
                .chain(
                    db.native_views
                        .iter()
                        .map(|v| (v.name.clone(), ObjectKind::View)),
                )
                .collect(),
        );
        section(
            SECTION_FORMS,
            db.forms
                .iter()
                .map(|f| (f.name.clone(), ObjectKind::Form))
                .collect(),
        );
        section(
            SECTION_REPORTS,
            db.reports
                .iter()
                .map(|r| (r.name.clone(), ObjectKind::Report))
                .collect(),
        );
        section(
            SECTION_MACROS,
            db.macros
                .iter()
                .map(|m| (m.name.clone(), ObjectKind::Macro))
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
                        .map(|m| (m.name.clone(), ObjectKind::Module))
                        .collect()
                })
                .unwrap_or_default(),
        );

        // Native Access forms/reports (phase 74, step 74.1; item M87):
        // catalog names only — read-only, no definition decoding, no tab.
        if !db.native_objects.is_empty() {
            let count = db.native_objects.len();
            ui.push_id("nav.native_objects", |ui| {
                egui::CollapsingHeader::new(
                    egui::RichText::new(tr(
                        "Native forms and reports ({count})",
                        &[("count", &count.to_string())],
                    ))
                    .strong(),
                )
                .default_open(true)
                .show(ui, |ui| {
                    for object in &db.native_objects {
                        ui.add_enabled(false, egui::Button::new(&object.name));
                    }
                    let hint = ui.weak(t(
                        "Definitions are not decoded — these objects are listed read-only.",
                    ));
                    crate::layout_probe::record("nav.native_objects_hint", hint.rect);
                });
            });
        }

        // A native-query decode failure is visible at the Queries group, not
        // silently swallowed (step 23.3, M10). Rendered after the sections so
        // the section closure's borrow of `ui` has ended.
        if let Some(reason) = &db.native_queries_error {
            ui.label(
                egui::RichText::new(reason)
                    .small()
                    .color(crate::theme::tokens(ui.visuals()).danger),
            );
        }
    });
    // The view entry point sits below the scrolling list (the section
    // closure holds the ui borrow inside the scroll area).
    if ui.small_button(t("New View")).clicked() {
        new_view = true;
    }
    if let Some(order) = sort_intent {
        app.nav_sort = order;
    }
    if new_view {
        app.open_view_editor(None);
    }
    if let Some((kind, name)) = delete_intent {
        app.delete_object(kind, &name);
    } else if let Some((kind, name)) = rename_intent {
        app.open_rename_dialog(ObjectId::new(kind, name));
    } else if let Some((kind, name, mode)) = open_intent {
        let object = ObjectId::new(kind, name);
        match mode {
            Some(mode) => app.dispatch(Command::OpenObjectInMode { object, mode }),
            None => app.dispatch(Command::OpenObject(object)),
        };
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
fn deletable(kind: ObjectKind) -> bool {
    !matches!(kind, ObjectKind::Macro | ObjectKind::Module)
}

/// Tables, queries, forms and reports rename (M81); the read-only
/// inventories and the native view editor (M93) keep their own surface.
fn renamable(kind: ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::Table | ObjectKind::Query | ObjectKind::Form | ObjectKind::Report
    )
}
