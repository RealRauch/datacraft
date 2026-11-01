//! Application frame and status bar (steps 5.1/12.4, items H17/H14).
//!
//! The four-region frame (ribbon, navigation pane, workspace, status bar)
//! and the status bar's record-navigation box. Pure paint over the snapshot
//! and the per-tab cursors.

use eframe::egui;

use datacraft_core::snapshot::UiStateSnapshot;

use crate::app::App;
use crate::i18n::{t, tr};
use crate::theme::tokens;

/// Status-line keys with a table count (the catalog parity test enumerates
/// them).
pub(crate) const STATUS_TABLE: &str = "{database} ({format}) — {count} table";
pub(crate) const STATUS_TABLES: &str = "{database} ({format}) — {count} tables";

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.capture_window_size(ui.ctx());
        crate::theme::apply(ui.ctx());
        let tokens = tokens(ui.visuals());

        egui::Panel::top("datacraft.ribbon")
            .frame(egui::Frame::side_top_panel(ui.style()).fill(tokens.ribbon))
            .show(ui, |ui| {
                crate::ribbon::ribbon_ui(self, ui);
            });

        if self.snapshot().nav_pane_visible {
            egui::Panel::left("datacraft.navpane")
                .resizable(true)
                .default_size(220.0)
                .frame(egui::Frame::side_top_panel(ui.style()).fill(tokens.panel))
                .show(ui, |ui| {
                    crate::nav_pane::nav_pane_ui(self, ui);
                });
        }

        egui::Panel::bottom("datacraft.status")
            .frame(egui::Frame::side_top_panel(ui.style()).fill(tokens.chrome))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left: database context (Access convention — the active
                    // document's object replaces the generic count).
                    let active = self.snapshot().active_tab.and_then(|id| {
                        self.snapshot()
                            .tabs
                            .iter()
                            .find(|t| t.id == id)
                            .map(|t| (t.object_kind.clone(), t.object_name.clone()))
                    });
                    ui.label(status_label(
                        self.snapshot(),
                        active.as_ref().map(|(k, n)| (k.as_str(), n.as_str())),
                    ));
                    if let Some(reason) = self.last_error() {
                        ui.colored_label(tokens.danger, reason);
                    } else if let Some(notice) = self.last_notice() {
                        ui.colored_label(tokens.accent, notice);
                    }
                    // Right: the record-navigation box over the active view's
                    // record domain (rows / results / form record / pages).
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.record_navigation_ui(ui);
                    });
                });
            });

        crate::workspace::workspace_ui(self, ui);

        crate::relationships::mapper_window(self, ui);
        crate::sql_window::sql_window(self, ui);
        crate::password_prompt::password_prompt(self, ui);
    }

    fn on_exit(&mut self) {
        // Settings are best-effort on shutdown: no surface is left to
        // report a failure.
        let _ = self.persist_settings();
    }
}

impl App {
    /// The status bar's record-navigation box (step 12.4): `◀ 3 of 12 ▶`
    /// over the active tab's record domain. Paint-only presentation state —
    /// each view kind maps to its own record cursor.
    fn record_navigation_ui(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.snapshot().clone();
        let Some(active) = snapshot
            .active_tab
            .and_then(|id| snapshot.tabs.iter().find(|t| t.id == id).cloned())
        else {
            return;
        };
        let (current, len) = match active.object_kind.as_str() {
            // Forms navigate their source rows (the maintain_caches pass
            // loads them per form tab); queries walk their result set.
            "query" => (
                self.record_cursor(active.id),
                self.results_for(active.id).map_or(0, |r| r.rows.len()),
            ),
            "report" => (
                self.preview_page(active.id),
                self.preview_pages(active.id).len(),
            ),
            _ => (
                self.form_record(active.id),
                self.rows_for(active.id).map_or(0, Vec::len),
            ),
        };
        if len == 0 {
            return;
        }
        let current = current.min(len - 1);
        if ui.small_button("\u{25c0}").clicked() && current > 0 {
            self.step_record(&active, current - 1, len);
        }
        ui.label(
            egui::RichText::new(tr(
                "{current} of {total}",
                &[
                    ("current", &(current + 1).to_string()),
                    ("total", &len.to_string()),
                ],
            ))
            .small(),
        );
        if ui.small_button("\u{25b6}").clicked() && current + 1 < len {
            self.step_record(&active, current + 1, len);
        }
    }

    /// Writes the stepped record position into the active view's cursor.
    fn step_record(&mut self, tab: &datacraft_core::snapshot::TabSnapshot, to: usize, len: usize) {
        match tab.object_kind.as_str() {
            "table" | "query" => self.set_record_cursor(tab.id, to, len),
            "form" => self.set_form_record(tab.id, to),
            "report" => self.set_preview_page(tab.id, to),
            _ => {}
        }
    }
}

/// Localized status-bar line (Access convention): built UI-side from the
/// locale-neutral snapshot — core's `status_line` stays English for
/// headless clients (ADR-4).
fn status_label(snapshot: &UiStateSnapshot, active: Option<(&str, &str)>) -> String {
    let Some(db) = &snapshot.database else {
        return t("No database open").to_owned();
    };
    let format = db.format.to_string();
    match active {
        Some((_, name)) => tr(
            "{database} ({format}) — {name}",
            &[("database", &db.name), ("format", &format), ("name", name)],
        ),
        None => {
            let count = db.tables.len();
            let key = if count == 1 {
                STATUS_TABLE
            } else {
                STATUS_TABLES
            };
            tr(
                key,
                &[
                    ("database", &db.name),
                    ("format", &format),
                    ("count", &count.to_string()),
                ],
            )
        }
    }
}
