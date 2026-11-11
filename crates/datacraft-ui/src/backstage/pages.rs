//! The backstage page bodies (step 21.3/29; card grammar step 38.2, M41).
//!
//! Every flow dispatches the same single commands as before; the templates
//! and samples render as cards with a painted mini-preview (the Excel
//! backstage grammar) instead of text lists.

use eframe::egui;
use eframe::egui::{Rect, Sense, Stroke, StrokeKind};

use datacraft_core::command::{Command, SourceSpec};

use crate::app::App;
use crate::backstage::BackstagePage;
use crate::backstage::recent::recent_databases_ui;
use crate::i18n::t;
use crate::theme::{fonts, tokens};
use crate::tooltips;

/// Right column: the selected page.
pub(super) fn page_body(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.heading(t(app.backstage_page.label()));
    ui.add_space(10.0);
    let mut intents: Vec<Command> = Vec::new();
    match app.backstage_page {
        BackstagePage::Home => home_page(app, ui, &mut intents),
        BackstagePage::New => new_database_page(app, ui, &mut intents),
        BackstagePage::Open => open_page(app, ui, &mut intents),
        BackstagePage::Samples => samples_page(app, ui, &mut intents),
        BackstagePage::Save => super::save::save_page(app, ui, &mut intents),
        BackstagePage::Info => super::info::info_page(app, ui),
        BackstagePage::Options => super::info::options_page(app, ui),
    }
    for command in intents {
        app.dispatch(command);
    }
}

/// Home: the open database's context plus the main flows.
fn home_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
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
    recent_databases_ui(app, ui, intents);
    status_line(app, ui);
}

/// New Database: pick a shipped template card and a target path; one click
/// dispatches create + open (phase 29, H44).
fn new_database_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let templates = app.snapshot().templates.clone();
    ui.label(t("Choose a template"));
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for template in &templates {
            let selected = app.new_database_template.as_deref() == Some(template.name.as_str());
            let caption = format!("{} — {} {}", template.title, template.tables, t("tables"));
            let clicked = card(
                ui,
                format!("backstage.new.{}", template.name),
                caption,
                template.tables > 0,
                selected,
            );
            if clicked {
                app.new_database_template = Some(template.name.clone());
            }
        }
    });
    ui.add_space(8.0);
    ui.label(t("path for the new database"));
    ui.add(
        egui::TextEdit::singleline(&mut app.new_database_path)
            .desired_width(360.0)
            .hint_text(t("path for the new database")),
    );
    let suggested = std::path::Path::new(app.new_database_path.trim())
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
        app.new_database_path = path.display().to_string();
    }
    ui.add_space(8.0);
    // The button gates on the composition's create capability
    // (step 42.2, L18); the dispatcher gate stays the second line.
    let creation_restriction = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::CREATE_DATABASE,
    )
    .copied();
    if let Some(reason) = creation_restriction {
        ui.weak(crate::availability::reason_text(&reason));
    }
    let enabled = creation_restriction.is_none()
        && !app.new_database_path.trim().is_empty()
        && app.new_database_template.is_some();
    let create = ui
        .push_id("backstage.new.create", |ui| {
            ui.add_enabled(enabled, egui::Button::new(t("Create Database")))
        })
        .inner;
    if tooltips::backstage_response(create, "backstage.new.create").clicked() {
        let path = app.new_database_path.clone();
        if let Some(template) = app.new_database_template.clone() {
            intents.push(Command::CreateDatabaseFromTemplate {
                path: path.clone().into(),
                template,
            });
            intents.push(Command::OpenDatabase {
                path: path.into(),
                password: None,
                source: None,
            });
        }
    }
    status_line(app, ui);
}

/// Open: the path buffer and the single open command.
/// The open entry accepts a path or a `file://` URI (step 65.3, item L8):
/// URI-shaped entries travel as [`SourceSpec::Uri`]; anything else keeps
/// the plain path shape.
fn open_source(entry: &str) -> Option<SourceSpec> {
    entry
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
        .then(|| SourceSpec::Uri(entry.to_owned()))
}

fn open_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let label = ui.label(t("Database path"));
    let field = ui.add(
        egui::TextEdit::singleline(&mut app.open_path)
            .desired_width(360.0)
            .hint_text(t("path or file:// URI")),
    );
    field.labelled_by(label.id);
    ui.horizontal(|ui| {
        let browse = ui
            .push_id("backstage.open.browse", |ui| ui.button(t("Browse…")))
            .inner;
        if tooltips::backstage_response(browse, "backstage.open.browse").clicked()
            && let Some(path) = app.dialogs.pick_open_file()
        {
            app.open_path = path.display().to_string();
        }
        let browse_folder = ui
            .push_id("backstage.open.browse_folder", |ui| {
                ui.button(t("Browse Folder…"))
            })
            .inner;
        if tooltips::backstage_response(browse_folder, "backstage.open.browse_folder").clicked()
            && let Some(path) = app.dialogs.pick_open_folder()
        {
            app.open_path = path.display().to_string();
        }
    });
    ui.add_space(8.0);
    let path = app.open_path.trim();
    let enabled = !path.is_empty();
    let open = ui
        .push_id("backstage.open.open_database", |ui| {
            ui.add_enabled(enabled, egui::Button::new(t("Open Database")))
        })
        .inner;
    if tooltips::backstage_response(open, "backstage.open.open_database").clicked() {
        // Server-source readiness (step 65.3, item L8): a `file://` entry
        // opens as a source spec — the URI travels as data and the core
        // resolves it; every other scheme answers the precise
        // no-server-driver boundary in the status line below.
        let source = open_source(app.open_path.trim());
        intents.push(Command::OpenDatabase {
            path: if source.is_some() {
                std::path::PathBuf::new()
            } else {
                app.open_path.clone().into()
            },
            password: None,
            source,
        });
    }
    recent_databases_ui(app, ui, intents);
    status_line(app, ui);
}

/// Samples: every database file of the resolved samples folder opens in
/// one click (phase 29, H44), rendered as cards.
fn samples_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let Some(dir) = samples_dir() else {
        ui.label(t("No samples found"));
        status_line(app, ui);
        return;
    };
    ui.label(format!("{}: {}", t("Samples folder"), dir.display()));
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for path in sample_files(&dir) {
            let name = path
                .file_name()
                .map_or_else(|| "?".to_owned(), |n| n.to_string_lossy().into_owned());
            if card(
                ui,
                format!("backstage.samples.{}", path.display()),
                name,
                true,
                false,
            ) {
                intents.push(Command::OpenDatabase {
                    path,
                    password: None,
                    source: None,
                });
            }
        }
    });
    status_line(app, ui);
}

/// One card: painted mini-preview over the caption, the whole card clickable
/// (Excel backstage grammar, step 38.2). Returns whether it was clicked.
fn card(
    ui: &mut egui::Ui,
    id_salt: String,
    caption: String,
    with_data: bool,
    selected: bool,
) -> bool {
    let tokens = tokens(ui.visuals());
    let size = egui::vec2(150.0, 106.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let id = ui.scope_id().with((id_salt.as_str(),));
    let response = ui.interact(rect, id, Sense::click());
    let clicked = response.clicked();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, caption.clone()));
    let preview = Rect::from_min_size(
        rect.min + egui::vec2(8.0, 8.0),
        egui::vec2(size.x - 16.0, 62.0),
    );
    ui.painter().rect_filled(preview, 2.0, tokens.canvas);
    ui.painter().rect_stroke(
        preview,
        2.0,
        Stroke::new(
            if selected { 1.6 } else { 1.0 },
            if selected {
                tokens.accent
            } else {
                tokens.border_strong
            },
        ),
        StrokeKind::Inside,
    );
    let band = Rect::from_min_max(
        preview.min + egui::vec2(6.0, 6.0),
        egui::pos2(preview.max.x - 6.0, preview.min.y + 16.0),
    );
    ui.painter().rect_filled(band, 1.0, tokens.accent_faint);
    preview_content(ui, preview, with_data);
    ui.painter().text(
        egui::pos2(rect.center().x, rect.max.y - 13.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(fonts::SMALL_SIZE),
        tokens.text,
    );
    clicked
}

/// The painted mini-preview content: hairline grid (blank) or accent bars
/// (a database with data), like the reference thumbnails.
fn preview_content(ui: &egui::Ui, preview: Rect, with_data: bool) {
    let tokens = tokens(ui.visuals());
    let body = Rect::from_min_max(
        egui::pos2(preview.min.x + 6.0, preview.min.y + 20.0),
        preview.max - egui::vec2(6.0, 6.0),
    );
    if with_data {
        for (index, (start, end)) in [(0.05, 0.55), (0.05, 0.8), (0.05, 0.4), (0.05, 0.65)]
            .into_iter()
            .enumerate()
        {
            let y = body.min.y + 6.0 + index as f32 * 9.0;
            ui.painter().rect_filled(
                Rect::from_min_max(
                    egui::pos2(body.min.x + (body.width() * start), y),
                    egui::pos2(body.min.x + (body.width() * end), y + 4.0),
                ),
                1.0,
                tokens.accent,
            );
        }
    } else {
        for fraction in [0.25, 0.5, 0.75] {
            let x = body.min.x + body.width() * fraction;
            ui.painter().line_segment(
                [egui::pos2(x, body.min.y), egui::pos2(x, body.max.y)],
                Stroke::new(1.0, tokens.border),
            );
        }
        for fraction in [0.33, 0.66] {
            let y = body.min.y + body.height() * fraction;
            ui.painter().line_segment(
                [egui::pos2(body.min.x, y), egui::pos2(body.max.x, y)],
                Stroke::new(1.0, tokens.border),
            );
        }
    }
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

/// One backstage flow button with its convention test id and the
/// registry tooltip (step 68.2, M74).
pub(super) fn backstage_button(
    app: &mut App,
    ui: &mut egui::Ui,
    control: &str,
    label: &str,
    enabled: bool,
    action: impl FnOnce(&mut App),
) {
    let full_id = format!("backstage.{}.{control}", app.backstage_page.slug());
    let response = ui
        .push_id(&full_id, |ui| {
            ui.add_enabled(enabled, egui::Button::new(label))
        })
        .inner;
    if tooltips::backstage_response(response, &full_id).clicked() {
        action(app);
    }
}

/// Last command error/notice at the bottom of the page.
pub(super) fn status_line(app: &App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    ui.add_space(10.0);
    if let Some(reason) = app.last_error() {
        ui.colored_label(tokens.danger, reason);
    } else if let Some(notice) = app.last_notice() {
        ui.colored_label(tokens.accent, notice);
    }
}
