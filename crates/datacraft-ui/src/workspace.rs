//! Tabbed document workspace (step 5.4, item H20; view modes in 21.1).
//!
//! Tab strip and central area render purely from the snapshot: tabs carry
//! their convention test ids (`workspace.tab.<kind>.<name>`) as push_id
//! salts, clicking activates (`SetActiveTab`), the ✕ closes
//! (`CloseTab`), and the central area shows the active tab's view — the
//! mode lives in the core and changes only through the ribbon's Views
//! split-button (step 21.1, U5). Intents are collected during paint and
//! applied after — no per-frame snapshot clones, no hidden state.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::state::TabMode;

use crate::app::App;
use crate::i18n::t;
use crate::theme::tokens;

/// Renders the tab strip and the central placeholder area.
pub fn workspace_ui(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    let empty = app.snapshot().tabs.is_empty();
    let no_database = app.snapshot().database.is_none();
    if empty {
        // The empty workspace still paints the canvas token — without this
        // the window's clear colour shows through (found in the Phase 20
        // gate, BACKLOG F3).
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(tokens.canvas))
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    if no_database {
                        ui.weak(t("Open a database from the File tab."));
                    } else {
                        ui.weak(t("No open documents"));
                    }
                });
            });
        return;
    }

    // Tab strip: collect intents, then apply after the borrow ends. The
    // row scrolls horizontally when many documents are open, and a
    // scroll-to-selected rule keeps the activated tab reachable (step
    // 88.2, item L45) — every activation path (nav pane, command,
    // keyboard flow) manifests as a changed active tab, so the rule
    // covers pointer and keyboard alike.
    let mut intent: Option<WorkspaceIntent> = None;
    egui::Panel::top("datacraft.tabstrip")
        .show_separator_line(false)
        .frame(egui::Frame::side_top_panel(ui.style()).fill(tokens.chrome))
        .show(ui, |ui| {
            let scroll_id = egui::Id::unique("datacraft.tabstrip.scroll");
            let activated = ui.ctx().data_mut(|data| {
                let last: Option<datacraft_core::ids::TabId> = data.get_temp(scroll_id);
                let active = app.snapshot().active_tab;
                if last != active {
                    data.insert_temp(scroll_id, active);
                    active
                } else {
                    None
                }
            });
            let strip = egui::ScrollArea::horizontal()
                .id_salt(scroll_id)
                .auto_shrink([false, true]);
            let out = strip.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let snapshot = app.snapshot();
                    let mut active_rect: Option<egui::Rect> = None;
                    for tab in &snapshot.tabs {
                        ui.push_id(&tab.ui_test_id, |ui| {
                            let is_active = Some(tab.id) == snapshot.active_tab;
                            crate::icons::icon(ui, &tab.object_kind, is_active);
                            let label = ui.selectable_label(is_active, &tab.object_name);
                            if label.clicked() {
                                intent = Some(WorkspaceIntent::Activate(tab.id));
                            }
                            if label.middle_clicked() {
                                intent = Some(WorkspaceIntent::Close(tab.id));
                            }
                            label.context_menu(|ui| {
                                if menu_item(
                                    ui,
                                    format!("{}.menu.close", tab.ui_test_id),
                                    t("Close"),
                                ) {
                                    intent = Some(WorkspaceIntent::Close(tab.id));
                                    ui.close();
                                }
                                if menu_item(
                                    ui,
                                    format!("{}.menu.close_others", tab.ui_test_id),
                                    t("Close Others"),
                                ) {
                                    intent = Some(WorkspaceIntent::CloseOthers(tab.id));
                                    ui.close();
                                }
                                // Close to one side (step 88.3, L49): the
                                // bus commands keep the anchor and answer
                                // an empty side precisely.
                                if menu_item(
                                    ui,
                                    format!("{}.menu.close_left", tab.ui_test_id),
                                    t("Close Tabs to the Left"),
                                ) {
                                    intent = Some(WorkspaceIntent::CloseLeft(tab.id));
                                    ui.close();
                                }
                                if menu_item(
                                    ui,
                                    format!("{}.menu.close_right", tab.ui_test_id),
                                    t("Close Tabs to the Right"),
                                ) {
                                    intent = Some(WorkspaceIntent::CloseRight(tab.id));
                                    ui.close();
                                }
                                if menu_item(
                                    ui,
                                    format!("{}.menu.close_all", tab.ui_test_id),
                                    t("Close All"),
                                ) {
                                    intent = Some(WorkspaceIntent::CloseAll);
                                    ui.close();
                                }
                            });
                            // Paints through the icon dispatcher (step 44.2,
                            // H53 sweep): the shipped font set has no "✕"
                            // glyph, so the text button rendered tofu in the
                            // live app and panicked the strict headless suite.
                            let (close_rect, close_response) = ui
                                .allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                            let close_response = close_response.on_hover_text(t("Close"));
                            let close_visuals = ui.style().interact(&close_response);
                            crate::icons::paint(
                                ui.painter(),
                                close_rect.shrink(4.0),
                                "close",
                                close_visuals.fg_stroke.color,
                                tokens.accent,
                            );
                            close_response.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Button, true, t("Close"))
                            });
                            if close_response.clicked() {
                                intent = Some(WorkspaceIntent::Close(tab.id));
                            }
                            let tab_rect = ui.min_rect();
                            crate::layout_probe::record(&tab.ui_test_id, tab_rect);
                            if is_active {
                                active_rect = Some(tab_rect);
                            }
                        });
                        ui.separator();
                    }
                    // Scroll-to-selected on activation (step 88.2, L45):
                    // only on a change of the active tab, so a manual
                    // scroll away never fights the user.
                    if let (Some(rect), true) = (active_rect, activated.is_some()) {
                        ui.scroll_to_rect(rect, None);
                    }
                });
            });
            crate::layout_probe::record("workspace.tabstrip.view", out.inner_rect);
            // The content area in screen space: the viewport origin minus
            // how far the strip already scrolled.
            let content_min = out.inner_rect.min - out.state.clamped_offset();
            crate::layout_probe::record(
                "workspace.tabstrip.content",
                egui::Rect::from_min_size(content_min, out.content_size),
            );
        });
    if let Some(applied) = intent {
        apply(app, applied);
    }

    // Central area renders the post-dispatch snapshot (immediate feedback).
    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(ui.style()).fill(tokens.canvas))
        .show(ui, |ui| {
            crate::layout_probe::record("shell.workspace_panel", ui.max_rect());
            let snapshot = app.snapshot().clone();
            let active = snapshot
                .active_tab
                .and_then(|id| snapshot.tabs.iter().find(|tab| tab.id == id).cloned());
            match active {
                Some(tab) if tab.object_kind == "form" => match tab.mode {
                    TabMode::Datasheet => crate::form_runtime::form_runtime_ui(app, &tab, ui),
                    TabMode::Design => crate::form_design::form_design_ui(app, &tab, ui),
                },
                Some(tab) if tab.object_kind == "report" => match tab.mode {
                    TabMode::Datasheet => crate::report_design::report_preview_ui(app, &tab, ui),
                    TabMode::Design => crate::report_design::report_design_ui(app, &tab, ui),
                },
                Some(tab) if tab.object_kind == "query" => match tab.mode {
                    TabMode::Datasheet => crate::datasheet::query_results_ui(app, &tab, ui),
                    TabMode::Design => crate::qbe::qbe_ui(app, &tab, ui),
                },
                Some(tab) if tab.object_kind == "view" => {
                    crate::views_window::view_ui(app, &tab, ui);
                }
                Some(tab) if tab.object_kind == "macro" => {
                    crate::macro_viewer::macro_viewer_ui(app, &tab, ui);
                }
                Some(tab) if tab.object_kind == "module" => {
                    crate::vba_viewer::vba_viewer_ui(app, &tab, ui);
                }
                Some(tab) => match tab.mode {
                    TabMode::Datasheet => crate::datasheet::datasheet_ui(app, &tab, ui),
                    TabMode::Design => crate::design::design_ui(app, &tab, ui),
                },
                None => {
                    ui.centered_and_justified(|ui| ui.weak(t("Select a tab")));
                }
            }
        });
}

enum WorkspaceIntent {
    Activate(TabId),
    Close(TabId),
    CloseOthers(TabId),
    CloseLeft(TabId),
    CloseRight(TabId),
    CloseAll,
}

/// One context-menu item with its convention test id.
fn menu_item(ui: &mut egui::Ui, id: String, label: &str) -> bool {
    ui.push_id(id, |ui| ui.button(label)).inner.clicked()
}

fn apply(app: &mut App, intent: WorkspaceIntent) {
    match intent {
        WorkspaceIntent::Activate(tab_id) => {
            app.dispatch(Command::SetActiveTab { tab_id });
        }
        WorkspaceIntent::Close(tab_id) => {
            app.dispatch(Command::CloseTab { tab_id });
        }
        WorkspaceIntent::CloseOthers(tab_id) => {
            app.dispatch(Command::CloseOtherTabs { tab_id });
        }
        WorkspaceIntent::CloseLeft(tab_id) => {
            app.dispatch(Command::CloseTabsToLeft { tab_id });
        }
        WorkspaceIntent::CloseRight(tab_id) => {
            app.dispatch(Command::CloseTabsToRight { tab_id });
        }
        WorkspaceIntent::CloseAll => {
            app.dispatch(Command::CloseAllTabs);
        }
    }
}
