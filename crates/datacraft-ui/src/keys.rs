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
        "Submit the staged datasheet batch or save the design sidecar (.datacraft).",
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
                // Ctrl+S on a table tab with staged edits submits the
                // pending batch (step 85.2, item H61) — the visible
                // pending state wins over the sidecar shortcut.
                if let Some(tab_id) = app.snapshot().active_tab
                    && app.has_staged_edits(tab_id)
                {
                    app.submit_datasheet_batch(tab_id);
                } else if !app.sidecar_path.trim().is_empty() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The command chords map the same actions where the redo chord is
    /// Ctrl+Y (step 92.1, item M99).
    #[test]
    fn command_chords_map_the_same_actions_elsewhere() {
        assert_eq!(
            shortcut(false, true, false, egui::Key::S),
            Some(Shortcut::SaveDesign)
        );
        assert_eq!(
            shortcut(false, true, false, egui::Key::W),
            Some(Shortcut::CloseTab)
        );
        assert_eq!(
            shortcut(false, true, false, egui::Key::Z),
            Some(Shortcut::Undo)
        );
        assert_eq!(
            shortcut(false, true, false, egui::Key::Y),
            Some(Shortcut::Redo)
        );
        assert_eq!(
            shortcut(false, false, false, egui::Key::S),
            None,
            "the save chord needs the command modifier"
        );
        assert_eq!(
            shortcut(false, false, false, egui::Key::Escape),
            Some(Shortcut::CancelEdit)
        );
    }

    /// macOS: ⌘S, ⌘W, ⌘Z undo, ⌘⇧Z redo; ⌘Y stays unmapped (step 92.1,
    /// item M99).
    #[test]
    fn macos_maps_the_command_chords_with_the_shift_z_redo() {
        assert_eq!(
            shortcut(true, true, false, egui::Key::S),
            Some(Shortcut::SaveDesign)
        );
        assert_eq!(
            shortcut(true, true, false, egui::Key::W),
            Some(Shortcut::CloseTab)
        );
        assert_eq!(
            shortcut(true, true, false, egui::Key::Z),
            Some(Shortcut::Undo)
        );
        assert_eq!(
            shortcut(true, true, true, egui::Key::Z),
            Some(Shortcut::Redo),
            "the macOS redo chord is ⌘⇧Z"
        );
        assert_eq!(
            shortcut(true, true, false, egui::Key::Y),
            None,
            "⌘Y stays unmapped on macOS"
        );
        assert_eq!(
            shortcut(true, false, false, egui::Key::S),
            None,
            "the save chord needs the command modifier"
        );
    }

    /// One label formatter for the help page and the tooltips: Ctrl+X on
    /// Windows/Linux (identity), ⌘X on macOS, and the redo chord renders
    /// ⌘⇧Z in both languages (step 92.1, item M99).
    #[test]
    fn shortcut_labels_render_the_command_form_on_macos() {
        assert_eq!(shortcut_label_with(false, "Ctrl+S"), "Ctrl+S");
        assert_eq!(
            shortcut_label_with(false, "Undo the last row write (Ctrl+Z)."),
            "Undo the last row write (Ctrl+Z)."
        );
        assert_eq!(shortcut_label_with(true, "Ctrl+S"), "⌘S");
        assert_eq!(shortcut_label_with(true, "Ctrl+Z"), "⌘Z");
        assert_eq!(shortcut_label_with(true, "Ctrl+Y"), "⌘⇧Z");
        assert_eq!(
            shortcut_label_with(true, "Undo the last row write (Ctrl+Z)."),
            "Undo the last row write (⌘Z)."
        );
        assert_eq!(
            shortcut_label_with(
                true,
                "Machen Sie die letzte Zeilenänderung rückgängig (Strg+Z)."
            ),
            "Machen Sie die letzte Zeilenänderung rückgängig (⌘Z)."
        );
        assert_eq!(
            shortcut_label_with(
                true,
                "Stellen Sie die zuletzt rückgängig gemachte Zeilenänderung wieder her (Strg+Y)."
            ),
            "Stellen Sie die zuletzt rückgängig gemachte Zeilenänderung wieder her (⌘⇧Z)."
        );
    }

    /// The macOS labels paint ⌘ and ⇧ through the shipped faces — the T11
    /// coverage rule applied to the generated labels.
    #[test]
    fn the_shipped_font_stack_paints_the_command_and_shift_glyphs() {
        let ctx = egui::Context::default();
        crate::theme::fonts::install(&ctx);
        ctx.begin_pass(egui::RawInput::default());
        let missing = ctx.fonts_mut(|fonts| {
            let font_id = egui::FontId::new(12.0, egui::FontFamily::Proportional);
            ['⌘', '⇧']
                .into_iter()
                .filter(|c| !fonts.has_glyph(&font_id, *c))
                .collect::<Vec<_>>()
        });
        assert!(
            missing.is_empty(),
            "the shipped stack misses shortcut glyphs: {missing:?}"
        );
    }
}
