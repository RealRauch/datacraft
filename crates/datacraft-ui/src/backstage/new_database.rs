//! The New Database page (step 75.2, item L42; format choice step 75.3,
//! item L43) — split out of `pages.rs` for the file-size gate (L30).
//!
//! Two steps in the Access shape: first *what* (a blank database or a
//! shipped template), then *where* (the path, and the format on the blank
//! path). The draft lives on `App` and survives Back; every choice still
//! dispatches one command chain.

use datacraft_core::command::Command;
use datacraft_core::port::DatabaseFormat;
use eframe::egui;

use crate::app::App;
use crate::i18n::t;
use crate::theme::fonts;
use crate::tooltips;

use super::pages::{backstage_button, card, status_line};

/// The step the New page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewDatabaseStep {
    /// Choose *what*: a blank database or one of the shipped templates.
    #[default]
    Kind,
    /// Choose *where*: the path (and the blank format) and Create.
    Location,
}

/// The chosen kind, kept while stepping back (step 75.2, L42).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewDatabaseKind {
    /// An empty database; the format is chosen on the location step.
    Blank,
    /// A shipped template by registry name (its format is template-defined).
    Template(String),
}

/// Presentation state of the New page (owned by `App` like the other
/// drafts; the commands still carry every decision).
#[derive(Debug, Clone)]
pub struct NewDatabaseDraft {
    /// The visible step.
    pub step: NewDatabaseStep,
    /// The kind chosen on step 1; `None` until a card is clicked.
    pub kind: Option<NewDatabaseKind>,
    /// Format of a blank database (step 75.3, L43); Jet4 is the default.
    pub blank_format: DatabaseFormat,
    /// Target path buffer.
    pub path: String,
}

impl Default for NewDatabaseDraft {
    fn default() -> Self {
        Self {
            step: NewDatabaseStep::Kind,
            kind: None,
            // `DatabaseFormat` has no `Default` impl (the core vocabulary is
            // explicit); the blank path starts in the Access-native family.
            blank_format: DatabaseFormat::Jet4,
            path: String::new(),
        }
    }
}

/// The page body: the capability notice plus the current step.
pub(super) fn new_database_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    // The create capability gates the page (step 42.2, L18); the reason
    // stands on both steps and the dispatcher gate stays the second line.
    let restriction = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::CREATE_DATABASE,
    )
    .copied();
    if let Some(reason) = restriction {
        ui.weak(crate::availability::reason_text(&reason));
        ui.add_space(6.0);
    }
    let show_location =
        app.new_database.step == NewDatabaseStep::Location && app.new_database.kind.is_some();
    if show_location {
        location_step(app, ui, intents, restriction.is_some());
    } else {
        kind_step(app, ui);
    }
    status_line(app, ui);
}

/// Step 1: the kind cards — a dedicated blank card plus the templates.
fn kind_step(app: &mut App, ui: &mut egui::Ui) {
    ui.label(t("Choose what to create"));
    ui.add_space(6.0);
    let blank_selected = app.new_database.kind == Some(NewDatabaseKind::Blank);
    // The registry's `blank` entry is the blank choice itself now; the
    // template list renders without it (step 75.2, L42).
    let templates: Vec<_> = app
        .snapshot()
        .templates
        .iter()
        .filter(|template| template.name != "blank")
        .cloned()
        .collect();
    let mut chosen: Option<NewDatabaseKind> = None;
    ui.horizontal_wrapped(|ui| {
        if card(
            ui,
            "backstage.new.blank".to_owned(),
            t("Blank database").to_owned(),
            false,
            blank_selected,
        ) {
            chosen = Some(NewDatabaseKind::Blank);
        }
        for template in &templates {
            let selected =
                app.new_database.kind == Some(NewDatabaseKind::Template(template.name.clone()));
            let caption = format!("{} — {} {}", template.title, template.tables, t("tables"));
            if card(
                ui,
                format!("backstage.new.{}", template.name),
                caption,
                template.tables > 0,
                selected,
            ) {
                chosen = Some(NewDatabaseKind::Template(template.name.clone()));
            }
        }
    });
    if let Some(kind) = chosen {
        app.new_database.kind = Some(kind);
        app.new_database.step = NewDatabaseStep::Location;
    }
}

/// Step 2: the chosen kind, the Back row, the path and Create.
fn location_step(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>, restricted: bool) {
    let Some(kind) = app.new_database.kind.clone() else {
        return;
    };
    let caption = match &kind {
        NewDatabaseKind::Blank => t("Blank database").to_owned(),
        NewDatabaseKind::Template(name) => app
            .snapshot()
            .templates
            .iter()
            .find(|template| template.name == *name)
            .map_or_else(|| name.clone(), |template| template.title.clone()),
    };
    ui.horizontal(|ui| {
        let back = ui
            .push_id("backstage.new.back", |ui| {
                ui.button(t("Back to the choice"))
            })
            .inner;
        if tooltips::backstage_response(back, "backstage.new.back").clicked() {
            app.new_database.step = NewDatabaseStep::Kind;
        }
        ui.add_space(10.0);
        ui.label(egui::RichText::new(caption).strong().size(fonts::BODY_SIZE));
    });
    ui.add_space(8.0);
    ui.label(t("path for the new database"));
    ui.add(
        egui::TextEdit::singleline(&mut app.new_database.path)
            .desired_width(360.0)
            .hint_text(t("path for the new database")),
    );
    let suggested = std::path::Path::new(app.new_database.path.trim())
        .file_name()
        .map_or_else(
            || "database.mdb".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
    let browse = ui
        .push_id("backstage.new.browse", |ui| ui.button(t("Browse…")))
        .inner;
    if tooltips::backstage_response(browse, "backstage.new.browse").clicked()
        && let Some(path) = app.dialogs.pick_save_file(&suggested)
    {
        app.new_database.path = path.display().to_string();
    }
    ui.add_space(8.0);
    let enabled = !restricted && !app.new_database.path.trim().is_empty();
    backstage_button(app, ui, "create", t("Create Database"), enabled, |app| {
        let path = app.new_database.path.trim().to_owned();
        match app.new_database.kind.clone() {
            Some(NewDatabaseKind::Blank) => intents.push(Command::CreateDatabaseFromTemplate {
                path: path.clone().into(),
                template: "blank".to_owned(),
            }),
            Some(NewDatabaseKind::Template(template)) => {
                intents.push(Command::CreateDatabaseFromTemplate {
                    path: path.clone().into(),
                    template,
                });
            }
            None => {}
        }
        intents.push(Command::OpenDatabase {
            path: path.into(),
            password: None,
            source: None,
        });
    });
}
