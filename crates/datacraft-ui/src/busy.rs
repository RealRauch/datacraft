//! Deferred busy dispatch (step 68.5, item M75).
//!
//! A command that may block runs one painted frame after its request, so
//! the localized overlay is on screen before the UI thread freezes. The
//! bus stays synchronous and single-session: no `std::thread`, no async
//! runtime (AGENTS §6). Only the three whole-file assets defer; `ImportFile`
//! and `RunSql` keep their synchronous event consumption at the call sites.

use datacraft_core::command::Command;
use eframe::egui;

use crate::app::App;
use crate::i18n::t;

/// Overlay label of the open flow.
pub const OPEN_LABEL: &str = "Opening the database…";
/// Overlay label of the create-from-template flow.
pub const CREATE_LABEL: &str = "Creating the database…";
/// Overlay label of the save-a-copy flow.
pub const SAVE_AS_LABEL: &str = "Writing the database copy…";

/// Every busy label, for the i18n coverage test.
pub fn all_labels() -> [&'static str; 3] {
    [OPEN_LABEL, CREATE_LABEL, SAVE_AS_LABEL]
}

/// Queued blocking commands with their overlay labels, oldest first.
#[derive(Debug, Default)]
pub struct BusyState {
    pending: std::collections::VecDeque<(Command, String)>,
    painted: bool,
}

impl App {
    /// Queues a command that may block; each one gets its own painted
    /// overlay frame before it runs (the New page queues create + open).
    pub fn dispatch_blocking(&mut self, command: Command, label: &'static str) {
        self.busy.pending.push_back((command, t(label).to_owned()));
        self.busy.painted = false;
    }

    /// Picks the oldest queued command once its overlay frame was painted.
    pub fn take_ready_command(&mut self) -> Option<Command> {
        if !self.busy.painted {
            return None;
        }
        self.busy.painted = false;
        self.busy.pending.pop_front().map(|(command, _)| command)
    }

    /// The overlay label of the next queued command.
    pub fn busy_label(&self) -> Option<&str> {
        self.busy.pending.front().map(|(_, label)| label.as_str())
    }

    /// Marks that the overlay for the next queued command is on screen.
    pub fn mark_busy_painted(&mut self) {
        if !self.busy.pending.is_empty() {
            self.busy.painted = true;
        }
    }
}

/// The deferrable actions' overlay labels; `None` for everything else.
pub fn command_label(command: &Command) -> Option<&'static str> {
    match command {
        Command::OpenDatabase { .. } => Some(OPEN_LABEL),
        Command::CreateDatabaseFromTemplate { .. } => Some(CREATE_LABEL),
        Command::SaveDatabaseAs { .. } => Some(SAVE_AS_LABEL),
        _ => None,
    }
}

/// Dispatches a command through the busy seam when it can block, directly
/// otherwise. Returns whether the command was queued.
pub fn dispatch_maybe_blocking(app: &mut App, command: Command) -> bool {
    match command_label(&command) {
        Some(label) => {
            app.dispatch_blocking(command, label);
            true
        }
        None => {
            let _ = app.dispatch(command);
            false
        }
    }
}

/// One frame of the busy seam: runs a ready command, then paints the
/// overlay for a pending one — that order is what puts the overlay on
/// screen before the blocking call freezes the thread.
pub fn busy_frame(app: &mut App, ui: &mut egui::Ui) {
    if let Some(command) = app.take_ready_command() {
        let _ = app.dispatch(command);
    }
    busy_overlay(app, ui);
}

/// Paints the overlay for a pending command and marks it as painted.
pub fn busy_overlay(app: &mut App, ui: &mut egui::Ui) {
    let Some(label) = app.busy_label().map(str::to_owned) else {
        return;
    };
    let screen = ui.ctx().content_rect();
    ui.ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("datacraft.busy.dim"),
        ))
        .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(60));
    egui::Window::new(t("Working…"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(label);
            });
        });
    ui.ctx().request_repaint();
    app.mark_busy_painted();
}
