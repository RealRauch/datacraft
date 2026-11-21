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
    // Access category header (step 21.5, U9): a plain labelled header
    // until the category model grows (L48) — a menu whose only entry
    // restates the selection cannot do anything (LESSONS #29).
    ui.strong(t("All Access Objects"));

    // The empty-shell early returns settle before the search field takes
    // its mutable borrow: the field renders only where it can do
    // something (LESSONS #29 — no dead controls).
    {
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
    }
    // Object search (step 92.3, item L48): a UI-owned presentation filter
    // — sections without a match hide, and a query without any match
    // shows one localized empty state.
    let query = crate::nav_search::search_field(app, ui);
    let searching = !query.trim().is_empty();
    let mut matched_any = false;
    let snapshot = app.snapshot();
    let Some(db) = &snapshot.database else {
        return; // the guard above settled an open database
    };
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
            entries.retain(|(name, _)| crate::nav_search::matches(&query, name));
            matched_any |= !entries.is_empty();
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
                            // The whole row is the click target (step 91.7,
                            // item L55): icon and label paint inside one
                            // interacted rect, like the backstage nav rows.
                            let row_height = ui
                                .spacing()
                                .interact_size
                                .y
                                .max(crate::icons::ICON_SIZE + 4.0);
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::hover(),
                            );
                            let row_id = ui.scope_id().with(("nav.row", &name));
                            let row = ui.interact(rect, row_id, egui::Sense::click());
                            row.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::Role::Button,
                                    true,
                                    is_active,
                                    name.clone(),
                                )
                            });
                            let tokens = crate::theme::tokens(ui.visuals());
                            if is_active {
                                ui.painter().rect_filled(rect, 0.0, tokens.accent_faint);
                            }
                            let icon_ink = if is_active {
                                tokens.accent_text
                            } else {
                                tokens.text_muted
                            };
                            let icon_rect = egui::Rect::from_min_size(
                                rect.left_top()
                                    + egui::vec2(4.0, (row_height - crate::icons::ICON_SIZE) / 2.0),
                                egui::vec2(crate::icons::ICON_SIZE, crate::icons::ICON_SIZE),
                            );
                            crate::icons::paint(
                                ui.painter(),
                                icon_rect,
                                kind.slug(),
                                icon_ink,
                                tokens.accent,
                            );
                            let font = egui::TextStyle::Body.resolve(ui.style());
                            let text_ink = if is_active {
                                tokens.accent_text
                            } else {
                                tokens.text
                            };
                            ui.painter().text(
                                rect.left_top()
                                    + egui::vec2(crate::icons::ICON_SIZE + 10.0, row_height / 2.0),
                                egui::Align2::LEFT_CENTER,
                                &name,
                                font,
                                text_ink,
                            );
                            if row.clicked() {
                                open_intent = Some((kind, name.clone(), None));
                            }
                            row.context_menu(|ui| {
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
                                        .push_id(format!("nav.{header}.{name}.menu.design"), |ui| {
                                            ui.button(t("Design"))
                                        })
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
                                        .push_id(format!("nav.{header}.{name}.menu.rename"), |ui| {
                                            match reason {
                                                Some(reason) => ui
                                                    .add_enabled(
                                                        false,
                                                        egui::Button::new(t("Rename")),
                                                    )
                                                    .on_hover_text(
                                                        crate::availability::reason_text(reason),
                                                    ),
                                                None => ui.button(t("Rename")),
                                            }
                                        })
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
                                        .push_id(format!("nav.{header}.{name}.menu.delete"), |ui| {
                                            match reason {
                                                Some(reason) => ui
                                                    .add_enabled(
                                                        false,
                                                        egui::Button::new(tr(
                                                            "Delete {name}",
                                                            &[("name", &name)],
                                                        )),
                                                    )
                                                    .on_hover_text(
                                                        crate::availability::reason_text(reason),
                                                    ),
                                                None => ui.button(tr(
                                                    "Delete {name}",
                                                    &[("name", &name)],
                                                )),
                                            }
                                        })
                                        .inner
                                        .clicked();
                                    if remove {
                                        delete_intent = Some((kind, name.clone()));
                                        ui.close();
                                    }
                                }
                            });
                        });
                    }
                });
            group.header_response.context_menu(|ui| {
                let by_name = ui
                    .push_id(format!("nav.{header}.menu.sort_name"), |ui| {
                        ui.selectable_label(
                            sort == NavSortOrder::Name,
                            t("Sort all sections by Name"),
                        )
                    })
                    .inner
                    .clicked();
                if by_name {
                    sort_intent = Some(NavSortOrder::Name);
                    ui.close();
                }
                let by_type = ui
                    .push_id(format!("nav.{header}.menu.sort_type"), |ui| {
                        ui.selectable_label(
                            sort == NavSortOrder::Type,
                            t("Sort all sections by Type"),
                        )
                    })
                    .inner
                    .clicked();
                if by_type {
                    sort_intent = Some(NavSortOrder::Type);
                    ui.close();
                }
                // View creation lives in the Views group menu (step 91.7,
                // item L58) — the pane bottom carries no detached button.
                if header == SECTION_VIEWS && ui.button(t("New View")).clicked() {
                    new_view = true;
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
        let native_matched: Vec<_> = db
            .native_objects
            .iter()
            .filter(|object| crate::nav_search::matches(&query, &object.name))
            .collect();
        matched_any |= !native_matched.is_empty();
        if !native_matched.is_empty() {
            let count = native_matched.len();
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
                    // The pane's row language (step 91.7, item L56): icon
                    // plus muted label, no button chrome — the read-only
                    // state carries the group hint, no affordance appears.
                    // The Label is a real widget, so the name stays in the
                    // accessibility tree.
                    let tokens = crate::theme::tokens(ui.visuals());
                    for object in &native_matched {
                        let slug = match object.kind {
                            datacraft_core::port::NativeObjectKind::Form => {
                                datacraft_core::ObjectKind::Form.slug()
                            }
                            datacraft_core::port::NativeObjectKind::Report => {
                                datacraft_core::ObjectKind::Report.slug()
                            }
                        };
                        ui.push_id(format!("nav.native.{name}", name = object.name), |ui| {
                            ui.horizontal(|ui| {
                                crate::icons::icon(ui, slug, false);
                                let width = ui.available_width();
                                ui.add_sized(
                                    [width, 0.0],
                                    egui::Label::new(
                                        egui::RichText::new(&object.name).color(tokens.text_muted),
                                    ),
                                );
                            });
                        });
                    }
                    let hint = ui.weak(t(
                        "Definitions are not decoded — these objects are listed read-only.",
                    ));
                    crate::layout_probe::record("nav.native_objects_hint", hint.rect);
                });
            });
        }

        // The empty state (step 92.3, item L48): a query that matches
        // nothing shows one localized line instead of a bare pane.
        if searching && !matched_any {
            ui.label(tr("No objects match {query}", &[("query", query.trim())]));
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
