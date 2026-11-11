//! Notice and error history (step 68.3, item M76).
//!
//! Every dispatch outcome lands in a bounded log: the status bar keeps
//! showing the latest line, and the messages dialog can retrieve the rest
//! (newest first). Entries carry a monotonic sequence instead of wall-clock
//! timestamps so tests stay deterministic.

use std::collections::VecDeque;

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
        let _ = (severity, text);
    }

    /// The newest entry.
    pub fn latest(&self) -> Option<&NoticeEntry> {
        None
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
        assert_eq!(log.latest().map(|entry| entry.severity), Some(Severity::Notice));

        log.clear();
        assert_eq!(log.entries().count(), 0, "clear empties the history");
        assert!(log.latest().is_none());
    }
}
