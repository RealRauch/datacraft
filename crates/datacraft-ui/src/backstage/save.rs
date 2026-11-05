//! Backstage Save-page copy/password flows (step 27.2, item M14) — split
//! from `backstage.rs` to keep every file under the size gate.

use eframe::egui;

use datacraft_core::command::Command;

use crate::app::App;
use crate::i18n::t;

use super::pages::status_line;

/// Save-a-copy flow (step 27.2, item M14): capability-gated through the
/// core availability map, one `SaveDatabaseAs` command per click.
fn save_copy_ui(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    ui.heading(t("Save a Copy"));
    let restriction =
        crate::availability::restricted(app.snapshot(), datacraft_core::availability::ids::SAVE_AS)
            .copied();
    if let Some(reason) = restriction {
        ui.weak(crate::availability::reason_text(&reason));
    }
    ui.add_enabled_ui(restriction.is_none(), |ui| {
        ui.label(t("path to the copy"));
        ui.add(
            egui::TextEdit::singleline(&mut app.save_as_path)
                .desired_width(360.0)
                .hint_text(t("path to the copy")),
        );
        let suggested = std::path::Path::new(app.save_as_path.trim())
            .file_name()
            .map_or_else(
                || "database.mdb".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
        if ui
            .push_id("backstage.save.browse", |ui| ui.button(t("Browse…")))
            .inner
            .clicked()
            && let Some(path) = app.dialogs.pick_save_file(&suggested)
        {
            app.save_as_path = path.display().to_string();
        }
        ui.checkbox(&mut app.save_as_encrypted, t("Protect with a password"));
        if app.save_as_encrypted {
            let label = ui.label(t("Password"));
            ui.add(
                egui::TextEdit::singleline(&mut app.save_as_password)
                    .password(true)
                    .desired_width(240.0),
            )
            .labelled_by(label.id);
        }
        let enabled = app.snapshot().database.is_some()
            && !app.save_as_path.trim().is_empty()
            && (!app.save_as_encrypted || !app.save_as_password.is_empty());
        if ui
            .push_id("backstage.save.save_copy", |ui| {
                ui.add_enabled(enabled, egui::Button::new(t("Save Copy")))
            })
            .inner
            .clicked()
            && overwrite_confirmed(app)
        {
            intents.push(Command::SaveDatabaseAs {
                path: app.save_as_path.trim().into(),
                password: app.save_as_encrypted.then(|| app.save_as_password.clone()),
            });
        }
    });
}

/// The save-copy overwrite gate (step 38.3, item M43): when the target file
/// already exists, a native confirmation decides — a declined dialog keeps
/// the existing file untouched.
fn overwrite_confirmed(app: &mut App) -> bool {
    let target = app.save_as_path.trim().to_owned();
    if !std::path::Path::new(&target).exists() {
        return true;
    }
    let message = crate::i18n::tr(
        "The file {path} already exists. Overwrite?",
        &[("path", &target)],
    );
    app.dialogs
        .confirm(crate::i18n::t("Overwrite File"), &message)
}

/// Change/remove-password flow (step 27.2, item M14).
fn change_password_ui(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    ui.heading(t("Database Password"));
    let restriction = crate::availability::restricted(
        app.snapshot(),
        datacraft_core::availability::ids::CHANGE_PASSWORD,
    )
    .copied();
    if let Some(reason) = restriction {
        ui.weak(crate::availability::reason_text(&reason));
    }
    ui.add_enabled_ui(restriction.is_none(), |ui| {
        let label = ui.label(t("New password"));
        ui.add(
            egui::TextEdit::singleline(&mut app.change_password)
                .password(true)
                .desired_width(240.0),
        )
        .labelled_by(label.id);
        ui.horizontal(|ui| {
            let can_change = app.snapshot().database.is_some() && !app.change_password.is_empty();
            if ui
                .push_id("backstage.save.change_password", |ui| {
                    ui.add_enabled(can_change, egui::Button::new(t("Change Password")))
                })
                .inner
                .clicked()
            {
                intents.push(Command::ChangePassword {
                    new_password: Some(app.change_password.clone()),
                });
            }
            let can_remove = app.snapshot().database.is_some();
            if ui
                .push_id("backstage.save.remove_password", |ui| {
                    ui.add_enabled(can_remove, egui::Button::new(t("Remove Password")))
                })
                .inner
                .clicked()
            {
                intents.push(Command::ChangePassword { new_password: None });
            }
        });
    });
}

/// Save: the sidecar path buffer and the save/load commands.
pub(super) fn save_page(app: &mut App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    ui.label(t("path to .datacraft"));
    ui.add(
        egui::TextEdit::singleline(&mut app.sidecar_path)
            .desired_width(360.0)
            .hint_text(t("path to .datacraft")),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let enabled = !app.sidecar_path.trim().is_empty();
        if ui
            .push_id("backstage.save.save_design", |ui| {
                ui.add_enabled(enabled, egui::Button::new(t("Save Design")))
            })
            .inner
            .clicked()
        {
            intents.push(Command::SaveDesignDocument {
                path: app.sidecar_path.clone().into(),
            });
        }
        if ui
            .push_id("backstage.save.load_design", |ui| {
                ui.add_enabled(enabled, egui::Button::new(t("Load Design")))
            })
            .inner
            .clicked()
        {
            intents.push(Command::LoadDesignDocument {
                path: app.sidecar_path.clone().into(),
            });
        }
    });
    ui.add_space(10.0);
    ui.separator();
    save_copy_ui(app, ui, intents);
    ui.add_space(10.0);
    ui.separator();
    change_password_ui(app, ui, intents);
    ui.add_space(10.0);
    ui.separator();
    close_database_ui(app, ui);
}

/// Closes the open database over the bus (step 44.3, item M52): the
/// user's route back to the no-database state so another file can open.
/// Capability-independent — closing needs nothing from the driver; the
/// button gates only on a database actually being open (the dispatcher
/// answers a precise failed-noop otherwise). Direct dispatch, since the
/// backstage page resets to Home as part of the same action.
fn close_database_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.label(t("Close the open database to open another one."));
    let enabled = app.snapshot().database.is_some();
    if ui
        .push_id("backstage.save.close_database", |ui| {
            ui.add_enabled(enabled, egui::Button::new(t("Close Database")))
        })
        .inner
        .clicked()
    {
        app.dispatch(Command::CloseDatabase);
        app.backstage_page = super::BackstagePage::Home;
    }
    status_line(app, ui);
}
