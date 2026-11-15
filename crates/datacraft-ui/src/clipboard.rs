//! The clipboard seam (step 86.2, item M101).
//!
//! The app speaks to the system clipboard through one port: the binary's
//! host provides [`SystemClipboard`] (arboard, opened per operation —
//! the clipboard is only touched on an actual copy/paste intent), the
//! headless suite injects a [`ScriptedClipboard`]. The TSV conversion
//! and validation live in core (`datacraft_core::clipboard`); this seam
//! only moves the text.

/// One clipboard port (the dialog-seam pattern, step 38.3, item M43).
pub trait ClipboardPort {
    /// The clipboard's text, or `None` when it is unavailable or holds
    /// none.
    fn get_text(&self) -> Option<String>;
    /// Writes text to the clipboard; `false` when unavailable.
    fn set_text(&self, text: String) -> bool;
}

/// The production host: arboard, opened per operation.
pub struct SystemClipboard;

impl ClipboardPort for SystemClipboard {
    fn get_text(&self) -> Option<String> {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.get_text().ok(),
            Err(_) => None,
        }
    }

    fn set_text(&self, text: String) -> bool {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.set_text(text).is_ok(),
            Err(_) => false,
        }
    }
}

/// The headless suite's scripted clipboard: one remembered text.
#[derive(Default)]
pub struct ScriptedClipboard {
    text: std::cell::RefCell<Option<String>>,
}

impl ScriptedClipboard {
    /// A clipboard pre-filled with `text`.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: std::cell::RefCell::new(Some(text.into())),
        }
    }
}

impl ClipboardPort for ScriptedClipboard {
    fn get_text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    fn set_text(&self, text: String) -> bool {
        *self.text.borrow_mut() = Some(text);
        true
    }
}
