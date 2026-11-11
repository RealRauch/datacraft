//! Contextual view switching (step 21.1, item U5).
//!
//! The Access Views split-button and the active-kind lookup that drives
//! contextual tabs; the mode itself lives in the core (`SetTabMode`).

use datacraft_core::command::Command;
use datacraft_core::state::TabMode;

use crate::app::App;
use crate::i18n::t;

use super::{ribbon_button, ribbon_button_disabled};

/// Object-kind slug of the active tab (contextual-tab selection and the
/// Views button's enablement).
pub(super) fn active_object_kind(app: &App) -> Option<String> {
    let snapshot = app.snapshot();
    let active = snapshot.active_tab?;
    snapshot
        .tabs
        .iter()
        .find(|tab| tab.id == active)
        .map(|tab| tab.object_kind.clone())
}

/// The Access Views split-button: the primary part switches to the other
/// view, the arrow opens the explicit view menu. Disabled unless the
/// active tab has a design view. `hover` is the localized registry
/// tooltip (step 68.2, M74).
pub(super) fn view_mode_button(
    ui: &mut eframe::egui::Ui,
    app: &App,
    tip: Option<&str>,
    hover: &str,
) -> Option<Command> {
    let snapshot = app.snapshot();
    let active = snapshot
        .active_tab
        .and_then(|id| snapshot.tabs.iter().find(|tab| tab.id == id));
    let Some(tab) = active.filter(|tab| tab.has_design_view) else {
        let disabled_hover = format!("{hover}\n{}", t("No active tab with a design view."));
        ribbon_button_disabled(ui, t("View"), "view_mode", tip, &disabled_hover);
        return None;
    };
    let current = tab.mode;
    let tab_id = tab.id;
    let mut intent = None;
    ui.horizontal(|ui| {
        if ribbon_button(ui, view_label(current), "view_mode", tip, hover) {
            intent = Some(Command::SetTabMode {
                tab_id,
                mode: current.toggled(),
            });
        }
        ui.menu_button("…", |ui| {
            for mode in [TabMode::Datasheet, TabMode::Design] {
                if ui
                    .selectable_label(mode == current, view_label(mode))
                    .clicked()
                {
                    intent = Some(Command::SetTabMode { tab_id, mode });
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(t("Switch view"));
    });
    intent
}

/// Localized view name (the English source string is the catalog key).
fn view_label(mode: TabMode) -> &'static str {
    match mode {
        TabMode::Datasheet => t("Datasheet"),
        TabMode::Design => t("Design"),
    }
}
