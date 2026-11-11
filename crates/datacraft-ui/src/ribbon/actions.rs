//! Ribbon action rendering (step 68.2, item M74; split from `ribbon.rs`
//! to keep the file below the size gate — L30).
//!
//! One action button: the spec state decides enabled/disabled rendering,
//! the tooltip registry supplies the hover sentence, and the paint code
//! collects at most one command per click.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::ribbon::{ActionState, RibbonAction};

use crate::app::App;
use crate::availability;
use crate::i18n::{t, tr};
use crate::tooltips;

use super::{command_for, ribbon_button, ribbon_button_disabled, views};

/// Renders one action and returns the command its click collected.
pub(super) fn action_button(
    app: &mut App,
    ui: &mut egui::Ui,
    tab_id: &str,
    group_id: &str,
    action: &RibbonAction,
    tip: Option<&str>,
) -> Option<Command> {
    let hover_key = tooltips::ribbon_tooltip(tab_id, group_id, action.id).unwrap_or(action.label);
    let mut intent = None;
    match &action.state {
        ActionState::Enabled if availability::restricted(app.snapshot(), action.id).is_some() => {
            // Capability-disabled for the open source (ADR-7): same muted
            // rendering as the planned actions, reason instead of phase.
            let reason =
                availability::restricted(app.snapshot(), action.id).expect("checked above");
            let hover = format!("{}\n{}", t(hover_key), availability::reason_text(reason));
            ribbon_button_disabled(ui, t(action.label), action.id, tip, &hover);
        }
        ActionState::Enabled => {
            if action.id == "table" {
                if ribbon_button(ui, t(action.label), action.id, tip, t(hover_key)) {
                    app.create_table_flow();
                }
            } else if action.id == "table_design" {
                if ribbon_button(ui, t(action.label), action.id, tip, t(hover_key)) {
                    app.create_table_design_flow();
                }
            } else if action.id == "open_database" {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.open_path)
                            .desired_width(180.0)
                            .hint_text(t("path to .mdb")),
                    );
                    if ribbon_button(ui, t(action.label), action.id, tip, t(hover_key)) {
                        intent = Some(Command::OpenDatabase {
                            path: app.open_path.clone().into(),
                            password: None,
                            source: None,
                        });
                    }
                });
            } else if action.id == "view_mode" {
                intent = views::view_mode_button(ui, app, tip, t(hover_key));
            } else if ribbon_button(ui, t(action.label), action.id, tip, t(hover_key))
                && let Some(command) = command_for(action.id, app)
            {
                intent = Some(command);
            }
        }
        ActionState::DisabledIn { phase } => {
            let hover = format!(
                "{}\n{}",
                t(hover_key),
                tr("arrives in {phase}", &[("phase", phase)])
            );
            ribbon_button_disabled(ui, t(action.label), action.id, tip, &hover);
        }
        ActionState::Planned => {
            let hover = format!("{}\n{}", t(hover_key), t("planned — not available yet"));
            ribbon_button_disabled(ui, t(action.label), action.id, tip, &hover);
        }
    }
    intent
}
