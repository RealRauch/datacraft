//! Keyboard shortcut contract tests (step 21.8, item U12; platform chords
//! step 92.1, item M99).
//!
//! The mapping is pure (`shortcut(macos, command, shift, key)`), so it is
//! testable without a window; `handle` only feeds egui's input state into
//! it. These rows pin the Windows/Linux form; the macOS chords carry their
//! own rows below.

use crate::keys::{Shortcut, shortcut};
use eframe::egui::Key;

#[test]
fn control_shortcuts_map_to_their_single_actions() {
    assert_eq!(
        shortcut(false, true, false, Key::S),
        Some(Shortcut::SaveDesign)
    );
    assert_eq!(
        shortcut(false, true, false, Key::W),
        Some(Shortcut::CloseTab)
    );
}

/// Step 68.1 (item M73): the undo/redo pair reaches the bus from the
/// keyboard — the commands already exist, only the keys were missing.
#[test]
fn undo_and_redo_map_to_the_history_commands() {
    assert_eq!(shortcut(false, true, false, Key::Z), Some(Shortcut::Undo));
    assert_eq!(shortcut(false, true, false, Key::Y), Some(Shortcut::Redo));
}

#[test]
fn undo_and_redo_without_the_command_modifier_are_ignored() {
    assert_eq!(shortcut(false, false, false, Key::Z), None);
    assert_eq!(shortcut(false, false, false, Key::Y), None);
}

/// Step 92.1 (item M99): the macOS chords — ⌘Z undoes, ⌘⇧Z redoes, and
/// ⌘Y stays unmapped where the redo chord convention is ⌘⇧Z.
#[test]
fn macos_chords_undo_with_command_z_and_redo_with_shift_command_z() {
    assert_eq!(shortcut(true, true, false, Key::Z), Some(Shortcut::Undo));
    assert_eq!(
        shortcut(true, true, true, Key::Z),
        Some(Shortcut::Redo),
        "the macOS redo chord is ⌘⇧Z"
    );
    assert_eq!(
        shortcut(true, true, false, Key::Y),
        None,
        "⌘Y stays unmapped on macOS"
    );
}

/// Step 68.4 (item M77): the help page's table cannot drift from the
/// mapping — every variant owns exactly one row.
#[test]
fn every_shortcut_has_exactly_one_help_row() {
    use crate::keys::SHORTCUT_ROWS;
    for shortcut in [
        Shortcut::SaveDesign,
        Shortcut::CloseTab,
        Shortcut::CancelEdit,
        Shortcut::Undo,
        Shortcut::Redo,
        Shortcut::ShortcutsHelp,
        Shortcut::Rename,
    ] {
        let rows = SHORTCUT_ROWS
            .iter()
            .filter(|(entry, _, _)| *entry == shortcut)
            .count();
        assert_eq!(rows, 1, "{shortcut:?} needs exactly one help row");
    }
    assert_eq!(
        SHORTCUT_ROWS.len(),
        7,
        "no help row exists without a shortcut"
    );
}

#[test]
fn f1_maps_to_the_shortcuts_help() {
    assert_eq!(
        shortcut(false, false, false, Key::F1),
        Some(Shortcut::ShortcutsHelp)
    );
}

/// Step 72.4 (item M81): F2 addresses the active object's rename dialog.
#[test]
fn f2_maps_to_the_rename_dialog() {
    assert_eq!(
        shortcut(false, false, false, Key::F2),
        Some(Shortcut::Rename)
    );
    assert_eq!(shortcut(false, true, false, Key::F2), None);
}

#[test]
fn escape_cancels_editing_without_a_modifier() {
    assert_eq!(
        shortcut(false, false, false, Key::Escape),
        Some(Shortcut::CancelEdit)
    );
    assert_eq!(shortcut(false, true, false, Key::Escape), None);
}

#[test]
fn unmodified_and_unrelated_keys_are_ignored() {
    assert_eq!(shortcut(false, false, false, Key::S), None);
    assert_eq!(shortcut(false, false, false, Key::W), None);
    assert_eq!(shortcut(true, true, false, Key::Q), None);
}
