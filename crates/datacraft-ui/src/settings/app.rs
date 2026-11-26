//! The App-facing settings surface (split from `settings.rs` for the
//! file-size gate, phase 102): applying loaded settings, persisting them
//! and the user-facing setters for language, appearance, low-power mode
//! and the print-preview zoom.

use std::path::{Path, PathBuf};

use crate::appearance::Appearance;

use super::{LanguageSetting, RecentDatabase, Settings, prune_notice, sane_preview_zoom};

impl crate::app::App {
    /// Attaches the UI settings file (the composition root sets this).
    pub fn set_settings_path(&mut self, path: Option<PathBuf>) {
        self.settings_path = path;
    }

    /// Attaches the dialog host: the binary uses the native dialogs, the
    /// headless suite injects scripted answers (step 38.3, item M43).
    pub fn set_dialogs(&mut self, host: Box<dyn crate::dialogs::DialogQueue>) {
        self.dialogs = host;
    }

    /// Applies persisted settings at startup: the stored language choice
    /// beats the environment and system detection (ADR-5).
    pub fn apply_settings(&mut self, settings: Settings) {
        if !settings.pruned_recent.is_empty() {
            self.record_notice(prune_notice(&settings.pruned_recent));
        }
        self.settings = settings;
        crate::i18n::init_language(self.language_setting().lang());
    }

    /// Recent database entries, newest first (item U15).
    pub fn recent_databases(&self) -> &[RecentDatabase] {
        self.settings.recent()
    }

    /// Current language choice.
    pub fn language_setting(&self) -> LanguageSetting {
        self.settings.language
    }

    /// Current appearance choice.
    pub fn appearance_setting(&self) -> Appearance {
        self.settings.appearance
    }

    /// Switches the appearance and persists the choice (ADR-6). The next
    /// frame applies it as egui's theme preference.
    pub fn set_appearance_setting(&mut self, setting: Appearance) {
        self.settings.appearance = setting;
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Current low-power rendering choice (`None` = auto-detection).
    pub fn low_power_setting(&self) -> Option<bool> {
        self.settings.low_power
    }

    /// Switches the low-power rendering choice and persists it (step
    /// 92.6, item L51). The mode itself resolves once at startup.
    pub fn set_low_power_setting(&mut self, choice: Option<bool>) {
        self.settings.low_power = choice;
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// The print-preview zoom of the report surface (phase 102, 102.4).
    pub fn preview_zoom(&self) -> f32 {
        self.settings.preview_zoom
    }

    /// Sets and persists the print-preview zoom (sanitized).
    pub fn set_preview_zoom(&mut self, zoom: f32) {
        self.settings.preview_zoom = sane_preview_zoom(Some(zoom));
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Switches the UI language live and persists the choice (ADR-5).
    pub fn set_language_setting(&mut self, setting: LanguageSetting) {
        self.settings.language = setting;
        crate::i18n::init_language(setting.lang());
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Records a successful open in the recent list and persists it
    /// (step 28.2, item U15).
    pub(crate) fn record_recent_database(&mut self, path: &Path) {
        self.settings.record_recent(path);
        if let Err(reason) = self.persist_settings() {
            self.record_error(crate::i18n::tr(
                "Settings could not be saved: {reason}",
                &[("reason", &reason.to_string())],
            ));
        }
    }

    /// Writes the settings file; `Ok(())` when none is attached.
    pub fn persist_settings(&self) -> std::io::Result<()> {
        match &self.settings_path {
            Some(path) => self.settings.save(path),
            None => Ok(()),
        }
    }

    /// Records the current viewport geometry for the next save.
    pub(crate) fn capture_window_geometry(&mut self, ctx: &eframe::egui::Context) {
        let viewport = ctx.input(|input| input.viewport().clone());
        let Some(size) = viewport.inner_rect.map(|rect| rect.size()) else {
            return;
        };
        let position = viewport.outer_rect.map(|rect| (rect.min.x, rect.min.y));
        let maximized = viewport.maximized.unwrap_or(false);
        self.settings
            .set_window(size.x, size.y, position, maximized);
    }

    /// Moves a window restored off-screen back onto the visible monitor,
    /// once per session (step 22.2, U13).
    pub(crate) fn rescue_window_position(&mut self, ctx: &eframe::egui::Context) {
        if self.position_rescued {
            return;
        }
        let Some(monitor) = ctx.input(|input| input.viewport().monitor_size) else {
            return;
        };
        self.position_rescued = true;
        let Some(geometry) = self.settings.window else {
            return;
        };
        let (Some(x), Some(y)) = (geometry.x, geometry.y) else {
            return;
        };
        let saved = eframe::egui::pos2(x, y);
        let size = eframe::egui::vec2(geometry.width, geometry.height);
        let target = crate::window::rescued_position(saved, size, monitor);
        if target != saved {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(target));
        }
    }
}
