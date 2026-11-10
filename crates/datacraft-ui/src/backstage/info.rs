//! The Info and Options backstage pages (split from `pages.rs` to stay
//! under the file-size gate; L30): database facts and the UI-only
//! language/appearance switchers (I5/U4).

use eframe::egui;

use crate::app::App;
use crate::appearance::Appearance;
use crate::i18n::{t, tr};
use crate::settings::LanguageSetting;

/// Info: database facts, read from the snapshot.
pub(super) fn info_page(app: &mut App, ui: &mut egui::Ui) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.label(t("No database open"));
        return;
    };
    let mut facts = vec![
        (t("Database"), db.name.clone()),
        (t("Path"), db.path.display().to_string()),
    ];
    // Server-source readiness (step 65.3, item L8): a URI session names
    // the spec it was opened with; plain path opens keep the facts list.
    if let Some(source) = &db.source {
        facts.push((t("Source"), source.to_string()));
    }
    facts.extend([
        (t("Format"), db.format.to_string()),
        (t("Tables"), db.tables.len().to_string()),
        (t("Relationships"), db.relationships.len().to_string()),
    ]);
    for (label, value) in facts {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong());
            ui.label(value);
        });
    }
    // Access co-existence (step 53.2, item M59): a detected lock file is
    // a database fact — the name plus the full localized remedy (the
    // same key the status bar's hover text carries).
    if let Some(file) = &db.lock_file {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t("Access lock file")).strong());
            ui.label(file);
        });
        ui.label(
            egui::RichText::new(tr(crate::shell::LOCK_NOTICE_DETAIL, &[("file", file)])).weak(),
        );
    }
}

/// Options: the UI-only language and appearance switchers (I5/U4).
pub(super) fn options_page(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(t("Language"));
        let current = app.language_setting();
        egui::ComboBox::from_id_salt("backstage.options.language")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for choice in [
                    LanguageSetting::Auto,
                    LanguageSetting::German,
                    LanguageSetting::English,
                ] {
                    if ui
                        .selectable_label(current == choice, choice.label())
                        .clicked()
                    {
                        app.set_language_setting(choice);
                    }
                }
            });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(t("Appearance"));
        let current = app.appearance_setting();
        egui::ComboBox::from_id_salt("backstage.options.appearance")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for choice in [Appearance::System, Appearance::Light, Appearance::Dark] {
                    if ui
                        .selectable_label(current == choice, choice.label())
                        .clicked()
                    {
                        app.set_appearance_setting(choice);
                    }
                }
            });
    });
}
