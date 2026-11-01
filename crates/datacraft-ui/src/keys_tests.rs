//! Keyboard shortcut contract tests (step 21.8, item U12).
//!
//! The mapping is pure (`shortcut(ctrl, key)`), so it is testable without a
//! window; `handle` only feeds egui's input state into it.

use crate::keys::{Shortcut, shortcut};
use eframe::egui::Key;

#[test]
fn control_shortcuts_map_to_their_single_actions() {
    assert_eq!(shortcut(true, Key::S), Some(Shortcut::SaveDesign));
    assert_eq!(shortcut(true, Key::W), Some(Shortcut::CloseTab));
}

#[test]
fn escape_cancels_editing_without_a_modifier() {
    assert_eq!(shortcut(false, Key::Escape), Some(Shortcut::CancelEdit));
    assert_eq!(shortcut(true, Key::Escape), None);
}

#[test]
fn unmodified_and_unrelated_keys_are_ignored() {
    assert_eq!(shortcut(false, Key::S), None);
    assert_eq!(shortcut(false, Key::W), None);
    assert_eq!(shortcut(true, Key::Q), None);
}
