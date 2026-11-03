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
        {
            intents.push(Command::SaveDatabaseAs {
                path: app.save_as_path.trim().into(),
                password: app.save_as_encrypted.then(|| app.save_as_password.clone()),
            });
        }
    });
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
    status_line(app, ui);
}
