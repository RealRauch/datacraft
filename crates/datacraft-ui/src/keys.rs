//! Keyboard handling (step 21.8, item U12).
//!
//! Ctrl+S, Ctrl+W and Esc map to the same single actions as their buttons.
//! Alt reveals ribbon key tips derived from the spec ids; only the model's
//! Tabs and Commands phases are taken (Legacy and Backstage are out of
//! scope). The shortcut mapping is pure and unit-tested.

use eframe::egui;

use datacraft_core::command::Command;
use datacraft_core::keys::key_tips;
use datacraft_core::ribbon::rendered_tabs;

use crate::app::App;

/// Current key-tip phase (the Tabs → Commands part of the model).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTipsPhase {
    /// Alt is down: tab tips are visible; a tip selects the tab.
    Tabs,
    /// A tab was selected: command tips are visible on its actions.
    Commands,
}

/// One keyboard shortcut, mapped to the same single action as its button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    /// Ctrl+S — saves the `.datacraft` design sidecar.
    SaveDesign,
    /// Ctrl+W — closes the active tab.
    CloseTab,
    /// Esc — cancels the buffered edit state and closes menus.
    CancelEdit,
}

/// Pure shortcut mapping: Ctrl+S, Ctrl+W, Esc (no modifier).
pub fn shortcut(ctrl: bool, key: egui::Key) -> Option<Shortcut> {
    if ctrl {
        return match key {
            egui::Key::S => Some(Shortcut::SaveDesign),
            egui::Key::W => Some(Shortcut::CloseTab),
            _ => None,
        };
    }
    match key {
        egui::Key::Escape => Some(Shortcut::CancelEdit),
        _ => None,
    }
}

/// Applies shortcuts and key-tip phases for the current frame.
pub fn handle(app: &mut App, ctx: &egui::Context) {
    let alt_down = ctx.input(|input| input.modifiers.alt);
    if alt_down && !app.alt_down {
        app.keytips = match app.keytips {
            None => Some(KeyTipsPhase::Tabs),
            Some(_) => None,
        };
    }
    app.alt_down = alt_down;

    let ctrl = ctx.input(|input| input.modifiers.ctrl);
    for key in [egui::Key::S, egui::Key::W, egui::Key::Escape] {
        if !ctx.input(|input| input.key_pressed(key)) {
            continue;
        }
        match shortcut(ctrl, key) {
            Some(Shortcut::SaveDesign) => {
                if !app.sidecar_path.trim().is_empty() {
                    app.dispatch(Command::SaveDesignDocument {
                        path: app.sidecar_path.clone().into(),
                    });
                }
            }
            Some(Shortcut::CloseTab) => {
                if let Some(tab_id) = app.snapshot().active_tab {
                    app.dispatch(Command::CloseTab { tab_id });
                }
            }
            Some(Shortcut::CancelEdit) => {
                app.keytips = None;
                app.cancel_edit();
            }
            None => {}
        }
    }

    if app.keytips == Some(KeyTipsPhase::Tabs) {
        select_tab_by_tip(app, ctx);
    }
}

/// Tabs phase: the first matching tab tip activates that ribbon tab and
/// moves to the command tips.
fn select_tab_by_tip(app: &mut App, ctx: &egui::Context) {
    let snapshot = app.snapshot();
    let active_kind = snapshot.active_tab.and_then(|id| {
        snapshot
            .tabs
            .iter()
            .find(|tab| tab.id == id)
            .map(|tab| tab.object_kind.clone())
    });
    let tabs = rendered_tabs(active_kind.as_deref());
    let ids: Vec<&str> = tabs.iter().map(|tab| tab.id).collect();
    let tips = key_tips(&ids);
    for (tab, tip) in tabs.iter().zip(tips) {
        let Some(key) = egui::Key::from_name(&tip) else {
            continue;
        };
        if ctx.input(|input| input.key_pressed(key)) {
            app.active_ribbon_tab = tab.id;
            app.keytips = Some(KeyTipsPhase::Commands);
            return;
        }
    }
}
