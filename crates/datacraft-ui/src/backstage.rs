//! File backstage (step 21.3, item U7; Excel-style chrome step 38.2, M41).
//!
//! Activating the File tab replaces the window content with a full-page
//! backstage: a full-height accent sidebar on the left (icon+label navigation
//! with the Options entry pinned to the bottom, like the Excel reference) and
//! the selected page on the right. Every flow here dispatches the same single
//! commands as before, and the path buffers stay on `App`.
//!
//! The pieces live beside this file: `nav` (the sidebar), `pages` (the page
//! bodies), `recent` (the recent-databases list) and `save` (save/copy and
//! the password flow).

use eframe::egui;

use crate::app::App;
use crate::theme::tokens;

mod nav;
mod pages;
pub(crate) mod recent;
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
    /// Pages in navigation order (the sidebar pins Options to the bottom).
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

    /// The sidebar glyph (`NAV_GLYPHS` vocabulary, step 38.1/38.2).
    pub fn glyph(self) -> &'static str {
        match self {
            BackstagePage::Home => "home",
            BackstagePage::New => "new",
            BackstagePage::Open => "open",
            BackstagePage::Samples => "database",
            BackstagePage::Save => "save",
            BackstagePage::Info => "info",
            BackstagePage::Options => "options",
        }
    }

    /// Whether the sidebar keeps this entry in the bottom-pinned group.
    pub fn pinned(self) -> bool {
        self == BackstagePage::Options
    }
}

/// Renders the backstage page for the File tab: the full-height accent
/// sidebar plus the selected page.
pub fn backstage_ui(app: &mut App, ui: &mut egui::Ui) {
    let tokens = tokens(ui.visuals());
    egui::Panel::left("datacraft.backstage.nav")
        .exact_size(220.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(tokens.accent_fill))
        .show(ui, |ui| nav::nav_ui(app, ui));
    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(ui.style()).fill(tokens.panel))
        .show(ui, |ui| pages::page_body(app, ui));
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

    /// The Excel grammar pins Options to the bottom of the sidebar; every
    /// other page stays in the main group (step 38.2, item M41).
    #[test]
    fn only_options_is_pinned_to_the_bottom() {
        for page in BackstagePage::ALL {
            assert_eq!(page.pinned(), page == BackstagePage::Options);
        }
    }

    /// Every page resolves to a sidebar glyph from the nav vocabulary.
    #[test]
    fn every_page_glyph_is_a_nav_glyph() {
        for page in BackstagePage::ALL {
            assert!(
                crate::icons::NAV_GLYPHS.contains(&page.glyph()),
                "page {:?} uses unknown glyph {:?}",
                page.slug(),
                page.glyph()
            );
        }
    }
}
