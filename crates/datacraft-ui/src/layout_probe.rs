//! Paint-rect recording for layout tests (step 44.1, items H52/T10).
//!
//! The semantic tree is blind to pixel geometry — AccessKit nodes exist
//! even when a widget paints off-window or under another panel, which
//! let H52 hide a whole ribbon group behind every tree pin. Paint sites
//! record their rects here under stable test ids; kittest suites drain
//! the log and assert geometry contracts. The log is a thread-local the
//! tests clear before and drain after the frame under test — cost in
//! the live app is a handful of pushes per frame. Supported testing
//! surface (like `MockPort` on the core crate — see docs/PUBLISHING.md).

use eframe::egui;
use std::cell::RefCell;

thread_local! {
    static RECTS: RefCell<Vec<(String, egui::Rect)>> = const { RefCell::new(Vec::new()) };
}

/// Records one paint rect under a stable test id (paint sites; also
/// callable from integration tests to mark probe points like the
/// window rect).
pub fn record(id: &str, rect: egui::Rect) {
    RECTS.with(|cell| cell.borrow_mut().push((id.to_owned(), rect)));
}

/// Clears the log; a test calls this inside the paint closure before the
/// frame under test renders.
pub fn clear() {
    RECTS.with(|cell| cell.borrow_mut().clear());
}

/// Drains the recorded rects; a test calls this after the frame ran.
pub fn drain() -> Vec<(String, egui::Rect)> {
    RECTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}
