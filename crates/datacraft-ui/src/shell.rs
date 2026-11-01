//! Application frame and status bar (steps 5.1/12.4, items H17/H14).
//!
//! The four-region frame (ribbon, navigation pane, workspace, status bar)
//! and the status bar's record-navigation box. Pure paint over the snapshot
//! and the per-tab cursors.

use eframe::egui;

use datacraft_core::present::status_line;

use crate::app::App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::theme::apply(ui.ctx());

        egui::Panel::top("datacraft.ribbon").show(ui, |ui| {
            crate::ribbon::ribbon_ui(self, ui);
        });

        if self.snapshot().nav_pane_visible {
            egui::Panel::left("datacraft.navpane")
                .resizable(true)
                .default_size(220.0)
                .show(ui, |ui| {
                    crate::nav_pane::nav_pane_ui(self, ui);
                });
        }

        egui::Panel::bottom("datacraft.status").show(ui, |ui| {
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
                ui.label(status_line(
                    self.snapshot(),
                    active.as_ref().map(|(k, n)| (k.as_str(), n.as_str())),
                ));
                if let Some(reason) = self.last_error() {
                    ui.colored_label(egui::Color32::from_rgb(0xB0, 0x2A, 0x2A), reason);
                } else if let Some(notice) = self.last_notice() {
                    ui.colored_label(crate::theme::TEAL, notice);
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
        ui.label(egui::RichText::new(format!("{} of {len}", current + 1)).small());
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
