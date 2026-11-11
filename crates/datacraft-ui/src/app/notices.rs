//! Notice and error history (step 68.3, item M76; semantic storage step
//! 70.2, item M86).
//!
//! Every dispatch outcome lands in a bounded log: the status bar keeps
//! showing the latest line, and the messages dialog can retrieve the rest
//! (newest first). Entries carry a monotonic sequence instead of wall-clock
//! timestamps so tests stay deterministic. Messages are stored semantically
//! (key plus arguments, or failure code plus reason) and rendered on
//! demand, so a live language switch re-renders the whole history.

use std::collections::VecDeque;

use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;

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

/// What a stored entry says, kept semantic so a live language switch can
/// re-render it (step 70.2, item M86).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeMessage {
    /// A catalog message: the i18n key plus its placeholder arguments.
    Key {
        /// English source key (the catalog lookup).
        key: &'static str,
        /// Placeholder name/value pairs.
        args: Vec<(String, String)>,
    },
    /// A failed command: the stable code plus the core's English reason.
    Failure {
        /// Stable failure class.
        code: FailureCode,
        /// Locatable core reason (English).
        reason: String,
    },
    /// A pre-rendered internal message without a catalog key (rules that
    /// build their text with `format!`).
    Literal(String),
}

impl NoticeMessage {
    /// Renders the message in the current UI language.
    pub fn text(&self) -> String {
        match self {
            Self::Key { key, args } => {
                let refs: Vec<(&str, &str)> = args
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str()))
                    .collect();
                tr(key, &refs)
            }
            Self::Failure { code, reason } => crate::failure_text::failure_text(*code, reason),
            Self::Literal(text) => text.clone(),
        }
    }

    /// Renders the message in an explicit language (deterministic tests —
    /// the live switch needs no global state).
    #[cfg(test)]
    pub(crate) fn text_in(&self, lang: crate::i18n::Lang) -> String {
        match self {
            Self::Key { key, args } => {
                let refs: Vec<(&str, &str)> = args
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str()))
                    .collect();
                crate::i18n::tr_in(lang, key, &refs)
            }
            Self::Failure { code, reason } => crate::failure_text::failure_text(*code, reason),
            Self::Literal(text) => text.clone(),
        }
    }
}

/// One history entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEntry {
    /// Failure or success notice.
    pub severity: Severity,
    /// Semantic message, rendered on demand.
    pub message: NoticeMessage,
    /// Monotonic sequence, higher = newer (stable UI ids).
    pub seq: u64,
}

impl NoticeEntry {
    /// The message in the current UI language.
    pub fn text(&self) -> String {
        self.message.text()
    }

    /// The message in an explicit language (deterministic tests).
    #[cfg(test)]
    pub(crate) fn text_in(&self, lang: crate::i18n::Lang) -> String {
        self.message.text_in(lang)
    }
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
    pub fn push(&mut self, severity: Severity, message: NoticeMessage) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push_front(NoticeEntry {
            severity,
            message,
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

/// The catalog keys a stored notice can carry; one const per key so the
/// catalog and `notice_for` cannot drift (step 70.2, item M86).
pub(crate) const NOTICE_COPY_SAVED: &str = "Copy saved to {path}";
/// Copy saved with a password.
pub(crate) const NOTICE_COPY_SAVED_ENCRYPTED: &str = "Copy saved to {path} (password-protected)";
/// Password set.
pub(crate) const NOTICE_PASSWORD_CHANGED: &str = "Database password changed";
/// Password removed.
pub(crate) const NOTICE_PASSWORD_REMOVED: &str = "Database password removed";
/// Design sidecar saved.
pub(crate) const NOTICE_DESIGN_SAVED: &str = "Design saved to {path}";
/// Design sidecar loaded.
pub(crate) const NOTICE_DESIGN_LOADED: &str =
    "Design loaded from {path} ({forms} forms, {queries} queries, {reports} reports)";

/// Every catalog key `notice_for` can store (i18n coverage test).
#[cfg(test)]
pub(crate) fn notice_keys() -> [&'static str; 6] {
    [
        NOTICE_COPY_SAVED,
        NOTICE_COPY_SAVED_ENCRYPTED,
        NOTICE_PASSWORD_CHANGED,
        NOTICE_PASSWORD_REMOVED,
        NOTICE_DESIGN_SAVED,
        NOTICE_DESIGN_LOADED,
    ]
}

/// Status-bar notice for successful design-document and database-copy
/// operations — stored as a catalog key plus arguments, so a language
/// switch re-renders it (step 70.2, item M86).
pub(crate) fn notice_for(events: &[Event]) -> Option<NoticeMessage> {
    events.iter().find_map(|event| match event {
        Event::DatabaseSavedAs { path, encrypted } => Some(NoticeMessage::Key {
            key: if *encrypted {
                NOTICE_COPY_SAVED_ENCRYPTED
            } else {
                NOTICE_COPY_SAVED
            },
            args: vec![("path".to_owned(), path.display().to_string())],
        }),
        Event::PasswordChanged { encrypted } => Some(NoticeMessage::Key {
            key: if *encrypted {
                NOTICE_PASSWORD_CHANGED
            } else {
                NOTICE_PASSWORD_REMOVED
            },
            args: Vec::new(),
        }),
        Event::DesignDocumentSaved { path } => Some(NoticeMessage::Key {
            key: NOTICE_DESIGN_SAVED,
            args: vec![("path".to_owned(), path.display().to_string())],
        }),
        Event::DesignDocumentLoaded {
            path,
            forms,
            queries,
            reports,
        } => Some(NoticeMessage::Key {
            key: NOTICE_DESIGN_LOADED,
            args: vec![
                ("path".to_owned(), path.display().to_string()),
                ("forms".to_owned(), forms.to_string()),
                ("queries".to_owned(), queries.to_string()),
                ("reports".to_owned(), reports.to_string()),
            ],
        }),
        _ => None,
    })
}

impl App {
    /// Reason of the last failed command, while the newest entry is one;
    /// rendered in the current UI language.
    pub fn last_error(&self) -> Option<String> {
        match self.notices.latest() {
            Some(entry) if entry.severity == Severity::Error => Some(entry.text()),
            _ => None,
        }
    }

    /// Success notice of the last command, while the newest entry is one;
    /// rendered in the current UI language.
    pub fn last_notice(&self) -> Option<String> {
        match self.notices.latest() {
            Some(entry) if entry.severity == Severity::Notice => Some(entry.text()),
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
        self.notices
            .push(Severity::Error, NoticeMessage::Literal(text));
    }

    /// Records one success notice on a non-dispatch path (settings load,
    /// `settings.rs`).
    pub(crate) fn record_notice(&mut self, text: String) {
        self.notices
            .push(Severity::Notice, NoticeMessage::Literal(text));
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
            log.push(
                Severity::Notice,
                NoticeMessage::Literal(format!("m{index}")),
            );
        }
        assert_eq!(log.entries().count(), NoticeLog::CAP, "the log is capped");
        assert_eq!(
            log.latest().map(NoticeEntry::text),
            Some("m54".to_owned()),
            "the newest entry is the last push"
        );
        let texts: Vec<String> = log.entries().map(NoticeEntry::text).collect();
        assert_eq!(
            texts.first().map(String::as_str),
            Some("m54"),
            "newest first"
        );
        assert_eq!(
            texts.last().map(String::as_str),
            Some("m5"),
            "the oldest entries were dropped"
        );
    }

    #[test]
    fn the_sequence_is_monotonic_and_clear_empties_the_log() {
        let mut log = NoticeLog::default();
        log.push(
            Severity::Error,
            NoticeMessage::Literal("first failure".into()),
        );
        log.push(
            Severity::Notice,
            NoticeMessage::Literal("then a success".into()),
        );
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

    /// A live language switch re-renders stored entries (step 70.2, M86):
    /// the log keeps the message identity, not the rendered text. The
    /// explicit render keeps the test independent of the global language.
    #[test]
    fn stored_messages_follow_a_live_language_switch() {
        use crate::i18n::Lang;

        let mut log = NoticeLog::default();
        log.push(
            Severity::Notice,
            NoticeMessage::Key {
                key: "Database password changed",
                args: Vec::new(),
            },
        );
        let entry = log.entries().next().expect("an entry");
        assert_eq!(entry.text_in(Lang::English), "Database password changed");
        assert_eq!(
            entry.text_in(Lang::German),
            "Datenbankpasswort geändert",
            "the stored message follows the switch"
        );
    }
}
