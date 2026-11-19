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
    // The product version (step 91.1, item M109) rides above the database
    // facts so it stays visible with and without an open database; the
    // label goes through `t()`, the value stays locale-neutral.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("Version")).strong());
        ui.label(datacraft_core::VERSION);
    });
    ui.add_space(6.0);
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.label(t("No database open"));
        return;
    };
    let mut facts: Vec<(String, String)> = vec![
        (t("Database").to_owned(), db.name.clone()),
        (t("Path").to_owned(), db.path.display().to_string()),
    ];
    // Server-source readiness (step 65.3, item L8): a URI session names
    // the spec it was opened with; plain path opens keep the facts list.
    if let Some(source) = &db.source {
        facts.push((t("Source").to_owned(), source.to_string()));
    }
    facts.extend([
        (t("Format").to_owned(), db.format.to_string()),
        (t("Tables").to_owned(), db.tables.len().to_string()),
        (
            t("Relationships").to_owned(),
            db.relationships.len().to_string(),
        ),
    ]);
    // Driver facts and the health verdict (phase 73, step 73.3; M91):
    // stable machine names map to localized labels; unknown names stay
    // verbatim. A damaged file names the exact reason.
    let driver_facts: Vec<(String, String)> = db
        .facts
        .iter()
        .map(|fact| (fact_label(&fact.name), fact.value.clone()))
        .collect();
    // Property-grid alignment (step 91.8, item L59): the widest label
    // measures the shared column edge; every value starts there instead
    // of following its own label's width. (Strong rendering widens the
    // glyphs a little — the slack absorbs it.)
    let label_column = facts
        .iter()
        .chain(driver_facts.iter())
        .map(|(label, _)| label_width(ui, label))
        .chain(std::iter::once(label_width(ui, t("Integrity check"))))
        .fold(0.0_f32, f32::max)
        + 12.0;
    for (label, value) in facts.iter().chain(driver_facts.iter()) {
        fact_row(ui, label_column, label, value, false);
    }
    fact_row(
        ui,
        label_column,
        t("Integrity check"),
        match &db.health {
            Some(reason) => reason.as_str(),
            None => t("ok"),
        },
        db.health.is_some(),
    );
    // Access co-existence (step 53.2, item M59): a detected lock file is
    // a database fact — the name plus the full localized remedy (the
    // same key the status bar's hover text carries).
    if let Some(file) = &db.lock_file {
        fact_row(ui, label_column, t("Access lock file"), file, false);
        ui.label(
            egui::RichText::new(tr(crate::shell::LOCK_NOTICE_DETAIL, &[("file", file)])).weak(),
        );
    }
}

/// One aligned fact row (step 91.8, item L59): the strong label, a
/// spacer to the shared column edge, then the value; the value rect
/// rides the layout probe under `backstage.info.fact_value`.
fn fact_row(ui: &mut egui::Ui, label_column: f32, label: &str, value: &str, danger: bool) {
    let tokens = crate::theme::tokens(ui.visuals());
    ui.horizontal(|ui| {
        let label_response = ui.label(egui::RichText::new(label).strong());
        let gap = label_column - label_response.rect.width();
        if gap > 0.0 {
            ui.allocate_exact_size(egui::vec2(gap, 0.0), egui::Sense::hover());
        }
        let value_response = if danger {
            ui.colored_label(tokens.danger, value)
        } else {
            ui.label(value)
        };
        crate::layout_probe::record("backstage.info.fact_value", value_response.rect);
    });
}

/// The rendered width of one label (the column measure).
fn label_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    ui.ctx().fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(label.to_owned(), font, egui::Color32::WHITE)
            .size()
            .x
    })
}

/// Localized label of one driver fact; unknown facts keep their wire name
/// (phase 73, step 73.3; M91).
fn fact_label(name: &str) -> String {
    match name {
        "sqlite_version" => t("SQLite version").to_owned(),
        "page_size" => t("Page size").to_owned(),
        "journal_mode" => t("Journal mode").to_owned(),
        "encoding" => t("Encoding").to_owned(),
        "user_version" => t("User version").to_owned(),
        "busy_timeout" => t("Busy timeout").to_owned(),
        "strict_tables" => t("STRICT tables").to_owned(),
        "without_rowid_tables" => t("WITHOUT ROWID tables").to_owned(),
        other => other.to_owned(),
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
