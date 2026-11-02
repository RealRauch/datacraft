//! File backstage (step 21.3, item U7).
//!
//! Activating the File tab replaces the window content with a full-page
//! backstage: page navigation on the left, the selected page on the right.
//! The old ribbon groups (and their inline path fields) are gone; every
//! flow here dispatches the same single commands as before, and the path
//! buffers stay on `App`.

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;
use crate::appearance::Appearance;
use crate::i18n::t;
use crate::settings::LanguageSetting;
use crate::theme::{fonts, tokens};

mod save;

/// Backstage pages — a deliberate subset of the Access set (Home, New,
/// Open, Samples, Save, Info, Options); Recent Databases is follow-up U15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackstagePage {
    /// Quick entry points and the open-database context.
    #[default]
    Home,
    /// Create a database from a shipped template (phase 29, H44).
    New,
    /// Open a database by path.
    Open,
    /// Open one of the shipped sample databases (phase 29, H44).
    Samples,
    /// Save/load the `.datacraft` design sidecar.
    Save,
    /// Database facts (name, path, format, table count).
    Info,
    /// UI-only language and appearance choices.
    Options,
}

impl BackstagePage {
    /// Pages in navigation order.
    pub const ALL: [BackstagePage; 7] = [
        BackstagePage::Home,
        BackstagePage::New,
        BackstagePage::Open,
        BackstagePage::Samples,
        BackstagePage::Save,
        BackstagePage::Info,
        BackstagePage::Options,
    ];

    /// Page caption (also the navigation label).
    pub fn label(self) -> &'static str {
        match self {
            BackstagePage::Home => "Home",
            BackstagePage::New => "New",
            BackstagePage::Open => "Open",
            BackstagePage::Samples => "Samples",
            BackstagePage::Save => "Save",
            BackstagePage::Info => "Info",
            BackstagePage::Options => "Options",
        }
    }

    /// Stable page id used in test-id salts (`backstage.<page>.<control>`).
    pub fn slug(self) -> &'static str {
        match self {
            BackstagePage::Home => "home",
            BackstagePage::New => "new",
            BackstagePage::Open => "open",
            BackstagePage::Samples => "samples",
            BackstagePage::Save => "save",
            BackstagePage::Info => "info",
            BackstagePage::Options => "options",
        }
    }
}

/// Renders the backstage page for the File tab.
pub fn backstage_ui(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(ui.style()).fill(tokens.canvas))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                backend_nav(app, ui);
                ui.separator();
                ui.vertical(|ui| page_body(app, ui));
            });
        });
}

/// Left column: back to the workspace, File caption, page list.
fn backend_nav(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    ui.vertical(|ui| {
        ui.set_width(190.0);
        ui.add_space(8.0);
        ui.push_id("backstage.back_to_workspace", |ui| {
            if ui.button(format!("← {}", t("Back"))).clicked() {
                app.active_ribbon_tab = "home";
            }
        });
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(t("File"))
                .strong()
                .size(fonts::HEADING_SIZE)
                .color(tokens.accent),
        );
        ui.add_space(8.0);
        for page in BackstagePage::ALL {
            let selected = app.backstage_page == page;
            ui.push_id(format!("backstage.nav.{}", page.slug()), |ui| {
                if ui.selectable_label(selected, t(page.label())).clicked() {
                    app.backstage_page = page;
                }
            });
        }
    });
}

/// Right column: the selected page.
fn page_body(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.heading(t(app.backstage_page.label()));
    ui.add_space(6.0);
    let mut intents: Vec<Command> = Vec::new();
    match app.backstage_page {
        BackstagePage::Home => home_page(app, ui),
        BackstagePage::New => new_database_page(app, ui, &mut intents),
        BackstagePage::Open => open_page(app, ui, &mut intents),
        BackstagePage::Samples => samples_page(app, ui, &mut intents),
        BackstagePage::Save => save::save_page(app, ui, &mut intents),
        BackstagePage::Info => info_page(app, ui),
        BackstagePage::Options => options_page(app, ui),
    }
    for command in intents {
        app.dispatch(command);
    }
}

/// Home: the open database's context plus the main flows.
fn home_page(app: &mut App, ui: &mut egui::Ui) {
    let database = app.snapshot().database.as_ref().map(|db| {
        (
            db.name.clone(),
            db.path.display().to_string(),
            db.format.to_string(),
            db.tables.len(),
        )
    });
    let has_database = database.is_some();
    match database {
        Some((name, path, format, tables)) => {
            ui.label(
                egui::RichText::new(format!("{name} ({format})"))
                    .strong()
                    .size(fonts::BODY_SIZE),
            );
            ui.label(egui::RichText::new(path).size(fonts::SMALL_SIZE));
            ui.label(
                egui::RichText::new(format!("{}: {tables}", t("Tables"))).size(fonts::SMALL_SIZE),
            );
        }
        None => {
            ui.label(t("No database open"));
        }
    }
    ui.add_space(12.0);
    backstage_button(app, ui, "open_page", t("Open Database"), true, |app| {
        app.backstage_page = BackstagePage::Open;
    });
    backstage_button(
        app,
        ui,
        "close_database",
        t("Close Database"),
        has_database,
        |app| {
            app.dispatch(Command::CloseDatabase);
        },
    );
    backstage_button(
        app,
        ui,
        "new_table",
        t("New Table"),
        has_database,
        App::create_table_flow,
    );
    backstage_button(app, ui, "save_page", t("Save Design"), true, |app| {
        app.backstage_page = BackstagePage::Save;
    });
    status_line(app, ui);
}

/// New Database: pick a shipped template and a target path; one click
/// dispatches create + open (phase 29, H44).
fn new_database_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let templates = app.snapshot().templates.clone();
    ui.label(t("Choose a template"));
    for template in &templates {
        let selected = app.new_database_template.as_deref() == Some(template.name.as_str());
        let label = format!("{} — {} {}", template.title, template.tables, t("tables"));
        if ui
            .push_id(format!("backstage.new.{}", template.name), |ui| {
                ui.selectable_label(selected, label)
            })
            .inner
            .clicked()
        {
            app.new_database_template = Some(template.name.clone());
        }
    }
    ui.add_space(8.0);
    ui.label(t("path for the new database"));
    ui.add(
        egui::TextEdit::singleline(&mut app.new_database_path)
            .desired_width(360.0)
            .hint_text(t("path for the new database")),
    );
    ui.add_space(8.0);
    let enabled = !app.new_database_path.trim().is_empty() && app.new_database_template.is_some();
    if ui
        .push_id("backstage.new.create", |ui| {
            ui.add_enabled(enabled, egui::Button::new(t("Create Database")))
        })
        .inner
        .clicked()
    {
        let path = app.new_database_path.clone();
        if let Some(template) = app.new_database_template.clone() {
            intents.push(Command::CreateDatabaseFromTemplate {
                path: path.clone().into(),
                template,
            });
            intents.push(Command::OpenDatabase {
                path: path.into(),
                password: None,
            });
        }
    }
    status_line(app, ui);
}

/// Open: the path buffer and the single open command.
fn open_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    ui.label(t("Database path"));
    ui.add(
        egui::TextEdit::singleline(&mut app.open_path)
            .desired_width(360.0)
            .hint_text(t("path to .mdb")),
    );
    ui.add_space(8.0);
    let path = app.open_path.trim();
    let enabled = !path.is_empty();
    if ui
        .push_id("backstage.open.open_database", |ui| {
            ui.add_enabled(enabled, egui::Button::new(t("Open Database")))
        })
        .inner
        .clicked()
    {
        intents.push(Command::OpenDatabase {
            path: app.open_path.clone().into(),
            password: None,
        });
    }
    status_line(app, ui);
}

/// Samples: every database file of the resolved samples folder opens in
/// one click (phase 29, H44).
fn samples_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let Some(dir) = samples_dir() else {
        ui.label(t("No samples found"));
        status_line(app, ui);
        return;
    };
    ui.label(format!("{}: {}", t("Samples folder"), dir.display()));
    ui.add_space(6.0);
    for path in sample_files(&dir) {
        let name = path
            .file_name()
            .map_or_else(|| "?".to_owned(), |n| n.to_string_lossy().into_owned());
        if ui
            .push_id(format!("backstage.samples.{}", path.display()), |ui| {
                ui.selectable_label(false, name)
            })
            .inner
            .clicked()
        {
            intents.push(Command::OpenDatabase {
                path,
                password: None,
            });
        }
    }
    status_line(app, ui);
}

/// Resolution order of the samples folder (L6): `DATACRAFT_SAMPLES_DIR`,
/// `samples/` next to the executable, the dev workspace.
fn samples_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("DATACRAFT_SAMPLES_DIR") {
        let dir = std::path::PathBuf::from(dir);
        if dir.is_dir() {
            return Some(dir);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let dir = parent.join("samples");
        if dir.is_dir() {
            return Some(dir);
        }
    }
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples");
    workspace.is_dir().then_some(workspace)
}

/// The database files of the samples folder, sorted by name.
fn sample_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("mdb") | Some("accdb") | Some("sqlite")
            )
        })
        .collect();
    files.sort();
    files
}

/// Info: database facts, read from the snapshot.
fn info_page(app: &mut App, ui: &mut egui::Ui) {
    let Some(db) = app.snapshot().database.as_ref() else {
        ui.label(t("No database open"));
        return;
    };
    let facts = [
        (t("Database"), db.name.clone()),
        (t("Path"), db.path.display().to_string()),
        (t("Format"), db.format.to_string()),
        (t("Tables"), db.tables.len().to_string()),
        (t("Relationships"), db.relationships.len().to_string()),
    ];
    for (label, value) in facts {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong());
            ui.label(value);
        });
    }
}

/// Options: the UI-only language and appearance switchers (I5/U4).
fn options_page(app: &mut App, ui: &mut egui::Ui) {
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

/// One backstage flow button with its convention test id.
fn backstage_button(
    app: &mut App,
    ui: &mut egui::Ui,
    control: &str,
    label: &str,
    enabled: bool,
    action: impl FnOnce(&mut App),
) {
    let clicked = ui
        .push_id(
            format!("backstage.{}.{control}", app.backstage_page.slug()),
            |ui| ui.add_enabled(enabled, egui::Button::new(label)),
        )
        .inner
        .clicked();
    if clicked {
        action(app);
    }
}

/// Last command error/notice at the bottom of the page.
fn status_line(app: &App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    ui.add_space(10.0);
    if let Some(reason) = app.last_error() {
        ui.colored_label(tokens.danger, reason);
    } else if let Some(notice) = app.last_notice() {
        ui.colored_label(tokens.accent, notice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_a_unique_slug_and_label() {
        let mut slugs = std::collections::BTreeSet::new();
        let mut labels = std::collections::BTreeSet::new();
        for page in BackstagePage::ALL {
            assert!(
                slugs.insert(page.slug()),
                "duplicate slug {:?}",
                page.slug()
            );
            assert!(
                labels.insert(page.label()),
                "duplicate label {:?}",
                page.label()
            );
        }
        assert_eq!(BackstagePage::default(), BackstagePage::Home);
    }
}
