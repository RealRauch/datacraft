//! Keyboard handling (step 21.8, item U12; undo/redo keys step 68.1, M73;
//! F1 help step 68.4, M77).
//!
//! Ctrl+S, Ctrl+W, Ctrl+Z, Ctrl+Y, Esc and F1 map to the same single
//! actions as their buttons. Alt reveals ribbon key tips derived from the
//! spec ids; only the model's Tabs and Commands phases are taken (Legacy
//! and Backstage are out of scope). The shortcut mapping is pure and
//! unit-tested.

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
    /// Ctrl+Z — undoes the last row write of the open database (step 68.1).
    Undo,
    /// Ctrl+Y — redoes the last undone row write (step 68.1).
    Redo,
    /// F1 — opens or closes the shortcuts help page (step 68.4).
    ShortcutsHelp,
    /// F2 — renames the active table/query/form/report (step 72.4, M81).
    Rename,
}

/// Every shortcut with its display keys and action description; the help
/// page renders exactly this table (step 68.4, item M77). The exhaustive
/// test keeps one row per variant.
pub const SHORTCUT_ROWS: &[(Shortcut, &str, &str)] = &[
    (
        Shortcut::SaveDesign,
        "Ctrl+S",
        "Save the design sidecar (.datacraft).",
    ),
    (Shortcut::CloseTab, "Ctrl+W", "Close the active tab."),
    (
        Shortcut::CancelEdit,
        "Esc",
        "Cancel the buffered edit or close menus.",
    ),
    (Shortcut::Undo, "Ctrl+Z", "Undo the last row write."),
    (Shortcut::Redo, "Ctrl+Y", "Redo the last undone row write."),
    (
        Shortcut::ShortcutsHelp,
        "F1",
        "Open or close this shortcuts page.",
    ),
    (
        Shortcut::Rename,
        "F2",
        "Rename the active table, query, form or report.",
    ),
];

/// Pure shortcut mapping: Ctrl+S, Ctrl+W, Ctrl+Z, Ctrl+Y, Esc, F1 (no
/// modifier).
pub fn shortcut(ctrl: bool, key: egui::Key) -> Option<Shortcut> {
    if ctrl {
        return match key {
            egui::Key::S => Some(Shortcut::SaveDesign),
            egui::Key::W => Some(Shortcut::CloseTab),
            egui::Key::Z => Some(Shortcut::Undo),
            egui::Key::Y => Some(Shortcut::Redo),
            _ => None,
        };
    }
    match key {
        egui::Key::Escape => Some(Shortcut::CancelEdit),
        egui::Key::F1 => Some(Shortcut::ShortcutsHelp),
        egui::Key::F2 => Some(Shortcut::Rename),
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
    for key in [
        egui::Key::S,
        egui::Key::W,
        egui::Key::Z,
        egui::Key::Y,
        egui::Key::Escape,
        egui::Key::F1,
        egui::Key::F2,
    ] {
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
            // The keyboard follows the QAT rule: an empty history is a
            // no-op, not a failure notice (step 68.1, M73).
            Some(Shortcut::Undo) => {
                if app.snapshot().undo_depth > 0 {
                    app.dispatch(Command::Undo);
                }
            }
            Some(Shortcut::Redo) => {
                if app.snapshot().redo_depth > 0 {
                    app.dispatch(Command::Redo);
                }
            }
            Some(Shortcut::CancelEdit) => {
                app.keytips = None;
                app.close_rename_dialog();
                app.cancel_edit();
            }
            Some(Shortcut::Rename) => {
                if let Some(object) = active_rename_target(app) {
                    app.open_rename_dialog(object);
                }
            }
            Some(Shortcut::ShortcutsHelp) => {
                if app.shortcuts_help_open() {
                    app.close_shortcuts_help();
                } else {
                    app.open_shortcuts_help();
                }
            }
            None => {}
        }
    }

    if app.keytips == Some(KeyTipsPhase::Tabs) {
        select_tab_by_tip(app, ctx);
    }
}

/// The active tab's object when its kind is renamable (step 72.4, M81):
/// F2 addresses the open document.
fn active_rename_target(app: &App) -> Option<datacraft_core::ObjectId> {
    let snapshot = app.snapshot();
    let active = snapshot.active_tab?;
    let tab = snapshot.tabs.iter().find(|tab| tab.id == active)?;
    let kind = datacraft_core::ObjectKind::from_slug(&tab.object_kind)?;
    matches!(
        kind,
        datacraft_core::ObjectKind::Table
            | datacraft_core::ObjectKind::Query
            | datacraft_core::ObjectKind::Form
            | datacraft_core::ObjectKind::Report
    )
    .then(|| datacraft_core::ObjectId::new(kind, &tab.object_name))
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
