//! Linked-table nav section (phase 100, step 100.3; item M131; roadmap §9
//! stage 0).
//!
//! The pane's row language for link records: an icon and a muted name in
//! the native-object style, the one-line target on a second line, and a
//! group hint that names the boundary. Nothing is clickable — a link has
//! no tab and no context menu — and the search matches the name and the
//! target.

use eframe::egui;

use datacraft_core::snapshot::LinkedTableSnapshot;

use crate::i18n::{t, tr};

/// Renders the link group; returns whether a row matched the query (the
/// pane's empty-state decision counts it).
pub(crate) fn links_section(ui: &mut egui::Ui, links: &[LinkedTableSnapshot], query: &str) -> bool {
    let matched: Vec<&LinkedTableSnapshot> = links
        .iter()
        .filter(|link| {
            crate::nav_search::matches(query, &link.name)
                || link
                    .target
                    .as_deref()
                    .is_some_and(|target| crate::nav_search::matches(query, target))
        })
        .collect();
    if matched.is_empty() {
        return false;
    }
    let count = matched.len().to_string();
    let tokens = crate::theme::tokens(ui.visuals());
    ui.push_id("nav.linked_tables", |ui| {
        egui::CollapsingHeader::new(
            egui::RichText::new(tr("Linked tables ({count})", &[("count", &count)])).strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            for link in &matched {
                ui.push_id(format!("nav.linked.{name}", name = link.name), |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::icon(ui, datacraft_core::ObjectKind::Table.slug(), false);
                        let width = ui.available_width();
                        ui.add_sized(
                            [width, 0.0],
                            egui::Label::new(
                                egui::RichText::new(&link.name).color(tokens.text_muted),
                            ),
                        );
                    });
                    if let Some(target) = &link.target {
                        ui.horizontal(|ui| {
                            ui.add_space(18.0);
                            let width = ui.available_width();
                            ui.add_sized(
                                [width, 0.0],
                                egui::Label::new(
                                    egui::RichText::new(tr(
                                        "Linked to {target}",
                                        &[("target", target)],
                                    ))
                                    .small()
                                    .color(tokens.text_muted),
                                ),
                            );
                        });
                    }
                });
            }
            ui.weak(t("Link records only — the targets are not resolved."));
        });
    });
    true
}
