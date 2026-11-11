//! Shortcuts-help open state (step 68.4, item M77).
//!
//! UI-owned flag; the page itself lives in `shortcuts_help.rs`.

use crate::app::App;

impl App {
    /// Opens the shortcuts help page.
    pub fn open_shortcuts_help(&mut self) {
        self.shortcuts_help_open = true;
    }

    /// Closes the shortcuts help page.
    pub fn close_shortcuts_help(&mut self) {
        self.shortcuts_help_open = false;
    }

    /// Whether the shortcuts help page is open.
    pub fn shortcuts_help_open(&self) -> bool {
        self.shortcuts_help_open
    }
}
