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
pub(crate) const SECTION_DETAILS: &str = "Object Details";

/// Renders the navigation pane contents (the panel itself is shown by the
/// shell only while `nav_pane_visible`).
pub fn nav_pane_ui(app: &mut App, ui: &mut egui::Ui) {
    // View dropdown (step 92.5, item L48 — the header the U9 note left
    // plain): the header switches between the category tree and the flat
    // Object Details list, persisted per database. The preferences clone
    // ends the borrow before anything paints.
    let db_key = app
        .snapshot()
        .database
        .as_ref()
        .map(|db| crate::nav_groups::prefs_key(&db.path));
    let prefs = db_key.as_deref().map(|key| app.nav_prefs(key));
    let view_intent = prefs
        .as_ref()
        .and_then(|prefs| crate::nav_groups::view_menu(ui, prefs));
    if db_key.is_none() {
        ui.strong(t("All Access Objects"));
    }

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
    let mut open_selection: Option<Vec<ObjectId>> = None;
    let mut new_group_intent: Option<Vec<ObjectId>> = None;
    let mut sort_intent: Option<NavSortOrder> = None;
    let mut new_view = false;
    // The selection the row clicks build (step 92.4, item L48): applied
    // after the scroll area, like every other intent.
    let mut selection_intent: Option<(Vec<ObjectId>, Option<ObjectId>)> = None;
    let current_selection = app.nav_selection.clone();
    let current_anchor = app.nav_anchor.clone();
    let mut groups_intent = crate::nav_groups::GroupsIntent::default();
    let prefs_for_groups = prefs.clone().unwrap_or_default();
    let details = prefs
        .as_ref()
        .is_some_and(|p| p.view == crate::nav_groups::NavView::Details);
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
                    for (index, (name, kind)) in entries.iter().enumerate() {
                        let name = name.clone();
                        let kind = *kind;
                        ui.push_id(format!("nav.{header}.{name}"), |ui| {
                            let is_active = active_object
                                .as_deref()
                                .is_some_and(|active| active.eq_ignore_ascii_case(&name));
                            let is_selected = current_selection
                                .iter()
                                .any(|id| id.kind == kind && id.name.eq_ignore_ascii_case(&name));
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
                            // The details view's trailing Type column (step
                            // 93.3, item L64): the localized kind rides the
                            // row's accessibility value so automation reads
                            // name and kind without changing the label.
                            let type_label =
                                details.then(|| crate::labels::object_kind_label(kind));
                            row.widget_info(|| {
                                let mut info = egui::WidgetInfo::selected(
                                    egui::Role::Button,
                                    true,
                                    is_active,
                                    name.clone(),
                                );
                                if let Some(type_label) = &type_label {
                                    info.current_text_value = Some((*type_label).to_owned());
                                }
                                info
                            });
                            crate::nav_row::paint_row(
                                ui,
                                rect,
                                row_height,
                                &name,
                                kind,
                                is_active || is_selected,
                                type_label,
                            );
                            if row.clicked() {
                                // Multi-select semantics (step 92.4, item
                                // L48): Shift ranges from the anchor within
                                // the section's display order, Ctrl toggles
                                // without opening, a plain click opens and
                                // resets the selection.
                                let (command_ctrl, shift) = ui.input(|input| {
                                    (
                                        input.modifiers.command || input.modifiers.ctrl,
                                        input.modifiers.shift,
                                    )
                                });
                                let clicked = ObjectId::new(kind, &name);
                                if shift {
                                    let ranged = current_anchor.as_ref().and_then(|anchor| {
                                        (anchor.kind == kind).then(|| {
                                            let anchor_index = entries
                                                .iter()
                                                .position(|(entry, _)| {
                                                    entry.eq_ignore_ascii_case(&anchor.name)
                                                })
                                                .unwrap_or(index);
                                            let (low, high) = if anchor_index <= index {
                                                (anchor_index, index)
                                            } else {
                                                (index, anchor_index)
                                            };
                                            entries[low..=high]
                                                .iter()
                                                .map(|(entry, entry_kind)| {
                                                    ObjectId::new(*entry_kind, entry)
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                    });
                                    selection_intent = Some((
                                        ranged.unwrap_or_else(|| vec![clicked.clone()]),
                                        current_anchor.clone(),
                                    ));
                                } else if command_ctrl {
                                    let mut picked = current_selection.clone();
                                    if let Some(existing) = picked
                                        .iter()
                                        .position(|id| id.kind == kind && id.name == name)
                                    {
                                        picked.remove(existing);
                                    } else {
                                        picked.push(clicked.clone());
                                    }
                                    selection_intent = Some((picked, current_anchor.clone()));
                                } else {
                                    open_intent = Some((kind, name.clone(), None));
                                    selection_intent = Some((vec![clicked.clone()], Some(clicked)));
                                }
                            }
                            row.context_menu(|ui| {
                                let multi = if is_selected && current_selection.len() > 1 {
                                    Some(current_selection.as_slice())
                                } else {
                                    None
                                };
                                let menu =
                                    crate::nav_menu::row_menu(app, ui, header, &name, kind, multi);
                                if let Some(ids) = menu.open_selection {
                                    open_selection = Some(ids);
                                }
                                if let Some(objects) = menu.new_group {
                                    new_group_intent = Some(objects);
                                }
                                if menu.open {
                                    open_intent = Some((kind, name.clone(), None));
                                }
                                if menu.design {
                                    open_intent = Some((kind, name.clone(), Some(TabMode::Design)));
                                }
                                if menu.rename {
                                    rename_intent = Some((kind, name.clone()));
                                }
                                if menu.delete {
                                    delete_intent = Some((kind, name.clone()));
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

        // The sections come from nav_sections (the details view, step
        // 92.5, item L48, lists every object in one flat section).
        let sections = if details {
            crate::nav_sections::details_sections(db)
        } else {
            crate::nav_sections::tree_sections(db)
        };
        for (header, entries) in sections {
            section(header, entries);
        }

        // Local groups (step 92.5, item L48): the user's custom groups
        // with their palette colours — the intents apply after the
        // snapshot borrow ends.
        let intent = crate::nav_groups::groups_section(ui, &prefs_for_groups, &query);
        matched_any |= intent.matched;
        groups_intent = intent;

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
    if let Some((picked, anchor)) = selection_intent {
        app.nav_selection = picked;
        app.nav_anchor = anchor;
    }
    if let Some(ids) = open_selection {
        for id in ids {
            app.dispatch(Command::OpenObject(id));
        }
    }
    if let Some(objects) = new_group_intent {
        app.open_group_dialog(objects);
    }
    if let (Some(view), Some(key)) = (view_intent, db_key.as_deref()) {
        app.set_nav_view(key, view);
    }
    if let Some((object, mode)) = groups_intent.open {
        match mode {
            Some(mode) => app.dispatch(Command::OpenObjectInMode { object, mode }),
            None => app.dispatch(Command::OpenObject(object)),
        };
    }
    if let (Some(key), Some((group, object))) = (&db_key, groups_intent.remove) {
        app.remove_from_nav_group(key, &group, &object);
    }
    if let Some(key) = &db_key {
        if let Some(group) = &groups_intent.delete_group {
            app.delete_nav_group(key, group);
        }
        if let Some((group, colour)) = &groups_intent.colour {
            app.set_nav_group_colour(key, group, *colour);
        }
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
