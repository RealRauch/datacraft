//! Notice and error history (step 68.3, item M76).
//!
//! Every dispatch outcome lands in a bounded log: the status bar keeps
//! showing the latest line, and the messages dialog can retrieve the rest
//! (newest first). Entries carry a monotonic sequence instead of wall-clock
//! timestamps so tests stay deterministic.

use std::collections::VecDeque;

use datacraft_core::event::Event;

use crate::app::App;
use crate::i18n::tr;

/// What the entry reports: a failed command or a success notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A command failed (`Event::Failed`).
    Error,
    /// A command succeeded with a user-facing notice.
    Notice,
}

/// One history entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEntry {
    /// Failure or success notice.
    pub severity: Severity,
    /// Localized message (the status bar's text).
    pub text: String,
    /// Monotonic sequence, higher = newer (stable UI ids).
    pub seq: u64,
}

/// Bounded newest-first history.
#[derive(Debug, Default)]
pub struct NoticeLog {
    entries: VecDeque<NoticeEntry>,
    next_seq: u64,
}

impl NoticeLog {
    /// The history cap.
    pub const CAP: usize = 50;

    /// Records one entry, dropping the oldest beyond the cap.
    pub fn push(&mut self, severity: Severity, text: String) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push_front(NoticeEntry {
            severity,
            text,
            seq,
        });
        self.entries.truncate(Self::CAP);
    }

    /// The newest entry.
    pub fn latest(&self) -> Option<&NoticeEntry> {
        self.entries.front()
    }

    /// All entries, newest first.
    pub fn entries(&self) -> impl Iterator<Item = &NoticeEntry> {
        self.entries.iter()
    }

    /// Drops the whole history.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Status-bar notice for successful design-document and database-copy
/// operations (moved here from `flows.rs`, step 68.3).
pub(crate) fn notice_for(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::DatabaseSavedAs { path, encrypted } => Some(if *encrypted {
            tr(
                "Copy saved to {path} (password-protected)",
                &[("path", &path.display().to_string())],
            )
        } else {
            tr(
                "Copy saved to {path}",
                &[("path", &path.display().to_string())],
            )
        }),
        Event::PasswordChanged { encrypted } => Some(if *encrypted {
            tr("Database password changed", &[])
        } else {
            tr("Database password removed", &[])
        }),
        Event::DesignDocumentSaved { path } => Some(tr(
            "Design saved to {path}",
            &[("path", &path.display().to_string())],
        )),
        Event::DesignDocumentLoaded {
            path,
            forms,
            queries,
            reports,
        } => Some(tr(
            "Design loaded from {path} ({forms} forms, {queries} queries, {reports} reports)",
            &[
                ("path", &path.display().to_string()),
                ("forms", &forms.to_string()),
                ("queries", &queries.to_string()),
                ("reports", &reports.to_string()),
            ],
        )),
        _ => None,
    })
}

impl App {
    /// Reason of the last failed command, while the newest entry is one.
    pub fn last_error(&self) -> Option<&str> {
        match self.notices.latest() {
            Some(entry) if entry.severity == Severity::Error => Some(&entry.text),
            _ => None,
        }
    }

    /// Success notice of the last command, while the newest entry is one.
    pub fn last_notice(&self) -> Option<&str> {
        match self.notices.latest() {
            Some(entry) if entry.severity == Severity::Notice => Some(&entry.text),
            _ => None,
        }
    }

    /// The bounded history, newest first.
    pub fn notice_entries(&self) -> impl Iterator<Item = &NoticeEntry> {
        self.notices.entries()
    }

    /// Drops the whole history.
    pub fn clear_notices(&mut self) {
        self.notices.clear();
    }

    /// Records one error on a non-dispatch failure path (the cell editor's
    /// stale-row guard, `flows.rs`).
    pub(crate) fn record_error(&mut self, text: String) {
        self.notices.push(Severity::Error, text);
    }

    /// Records one success notice on a non-dispatch path (settings load,
    /// `settings.rs`).
    pub(crate) fn record_notice(&mut self, text: String) {
        self.notices.push(Severity::Notice, text);
    }

    /// Opens the messages dialog.
    pub fn open_notices(&mut self) {
        self.notices_open = true;
    }

    /// Closes the messages dialog.
    pub fn close_notices(&mut self) {
        self.notices_open = false;
    }

    /// Whether the messages dialog is open.
    pub fn notices_open(&self) -> bool {
        self.notices_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_newest_first_and_bounded() {
        let mut log = NoticeLog::default();
        for index in 0..(NoticeLog::CAP + 5) {
            log.push(Severity::Notice, format!("m{index}"));
        }
        assert_eq!(log.entries().count(), NoticeLog::CAP, "the log is capped");
        assert_eq!(
            log.latest().map(|entry| entry.text.as_str()),
            Some("m54"),
            "the newest entry is the last push"
        );
        let texts: Vec<&str> = log.entries().map(|entry| entry.text.as_str()).collect();
        assert_eq!(texts.first(), Some(&"m54"), "newest first");
        assert_eq!(texts.last(), Some(&"m5"), "the oldest entries were dropped");
    }

    #[test]
    fn the_sequence_is_monotonic_and_clear_empties_the_log() {
        let mut log = NoticeLog::default();
        log.push(Severity::Error, "first failure".into());
        log.push(Severity::Notice, "then a success".into());
        let seqs: Vec<u64> = log.entries().map(|entry| entry.seq).collect();
        assert_eq!(seqs.len(), 2);
        assert!(seqs[0] > seqs[1], "newest carries the higher sequence");
        assert_eq!(
            log.latest().map(|entry| entry.severity),
            Some(Severity::Notice)
        );

        log.clear();
        assert_eq!(log.entries().count(), 0, "clear empties the history");
        assert!(log.latest().is_none());
    }

    /// Every dispatch outcome lands in the history; the status accessors
    /// report the newest entry only (step 68.3, item M76).
    #[test]
    fn dispatch_feeds_the_history_and_the_status_accessors() {
        use datacraft_core::command::Command;
        use datacraft_core::port::MockPort;

        let mut app = App::new(Box::new(MockPort::new()));
        let _ = app.dispatch(Command::Undo);
        assert!(
            app.last_error().is_some(),
            "a failed command is the latest entry"
        );
        assert!(app.last_notice().is_none());
        assert_eq!(app.notice_entries().count(), 1);

        let _ = app.dispatch(Command::Undo);
        assert_eq!(
            app.notice_entries().count(),
            2,
            "the second failure is appended, not replacing the first"
        );
        let seqs: Vec<u64> = app.notice_entries().map(|entry| entry.seq).collect();
        assert!(seqs[0] > seqs[1], "newest first");

        app.close_notices();
        assert!(!app.notices_open());
        app.open_notices();
        assert!(app.notices_open());
    }
}
