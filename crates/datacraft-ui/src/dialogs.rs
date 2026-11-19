//! Native file and message dialogs (step 38.3, item M43; the queued
//! model is step 81.1, item M97).
//!
//! `rfd` drives the platform dialogs (Windows common dialogs, NSOpenPanel,
//! GTK portals). The seam is the **queued** [`DialogQueue`] (M97): a flow
//! parks a [`DialogRequest`] and receives the [`DialogAnswer`] through
//! `poll_answer` on a later frame, so no call site ever blocks inside
//! a frame. The native host runs rfd's async dialogs (non-modal
//! panels) on a helper thread through the minimal `std::task::Wake`
//! executor in `block_on` — no async runtime, no new dependency —
//! because a synchronous `runModal` inside winit's event handler is
//! the documented macOS abort class (craft-libs' `craft-dialogs`).
//!
//! Two fences, decided in M43: native dialogs speak the OS language (outside
//! the i18n catalog by design — Excel behaves the same), while the prompt
//! texts we render ourselves go through `t()`/`tr()`; and the "No COM"
//! principle refers to the data path, not OS shell integration (rfd drives
//! the Windows common-dialog API the same way winit drives the windowing
//! API).

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::task::{Context, Poll, Wake, Waker};

/// A dialog request parked with the queue (step 81.1, item M97): the
/// host answers it through `poll_answer` on a later frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogRequest {
    /// A native open-file picker; `extensions` names the supported
    /// suffixes (without dots) the picker filters to (step 91.3, M111).
    OpenFile { extensions: &'static [&'static str] },
    /// A native open-folder picker (the CSV directory driver opens a
    /// directory as a database).
    OpenFolder,
    /// A native save-file picker seeded with `suggested`.
    SaveFile { suggested: String },
    /// A native Ok/Cancel confirmation for a destructive action.
    Confirm { title: String, message: String },
}

/// The answer to a [`DialogRequest`] (step 81.1, item M97).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAnswer {
    /// A picked path; `None` is the user's Cancel.
    Path(Option<PathBuf>),
    /// The confirmation answer (true = Ok/Yes).
    Confirmed(bool),
    /// The host could not produce an answer at all — distinct from
    /// Cancel, so a flow can report a precise failure instead of
    /// silently pretending the user declined.
    Unavailable,
}

/// The queued dialog seam (step 81.1, item M97): requests park, answers
/// arrive on a later frame — no call site blocks inside a frame.
pub trait DialogQueue {
    /// Parks one request; the answer arrives through [`DialogQueue::poll_answer`].
    fn request(&mut self, request: DialogRequest);
    /// Pops one completed answer, if any.
    fn poll_answer(&mut self) -> Option<DialogAnswer>;
}

/// The real host: platform dialogs through `rfd`'s async API, answered
/// on a helper thread so the frame never blocks (step 81.1, item M97).
///
/// Every request spawns one short-lived thread that drives the dialog's
/// future through `block_on` and mails the answer back; the UI polls
/// the channel once per frame. The dialog itself is presented by rfd's
/// per-platform async backend (a non-modal panel), so no modal loop is
/// ever nested inside winit's event handler — the documented macOS
/// abort class cannot reenter the frame.
#[derive(Debug, Default)]
pub struct NativeDialogs {
    answers: Option<(mpsc::Sender<DialogAnswer>, mpsc::Receiver<DialogAnswer>)>,
}

impl NativeDialogs {
    /// Answers `request` on the calling thread — the helper-thread body.
    /// The blocking happens here, away from the event loop.
    fn answer_on_thread(request: DialogRequest) -> DialogAnswer {
        match request {
            DialogRequest::OpenFile { extensions } => DialogAnswer::Path(
                block_on(
                    rfd::AsyncFileDialog::new()
                        .add_filter(crate::i18n::t("Supported files"), extensions)
                        .pick_file(),
                )
                .map(|handle| handle.path().to_path_buf()),
            ),
            DialogRequest::OpenFolder => DialogAnswer::Path(
                block_on(rfd::AsyncFileDialog::new().pick_folder())
                    .map(|handle| handle.path().to_path_buf()),
            ),
            DialogRequest::SaveFile { suggested } => DialogAnswer::Path(
                block_on(
                    rfd::AsyncFileDialog::new()
                        .set_file_name(suggested)
                        .save_file(),
                )
                .map(|handle| handle.path().to_path_buf()),
            ),
            DialogRequest::Confirm { title, message } => DialogAnswer::Confirmed(matches!(
                block_on(
                    rfd::AsyncMessageDialog::new()
                        .set_title(&title)
                        .set_description(&message)
                        .set_buttons(rfd::MessageButtons::OkCancel)
                        .set_level(rfd::MessageLevel::Warning)
                        .show()
                ),
                rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes
            )),
        }
    }
}

impl DialogQueue for NativeDialogs {
    fn request(&mut self, request: DialogRequest) {
        let channel = self.answers.get_or_insert_with(mpsc::channel);
        let sender = channel.0.clone();
        let spawned = std::thread::Builder::new()
            .name("datacraft-dialog".to_owned())
            .spawn(move || {
                let _ = sender.send(NativeDialogs::answer_on_thread(request));
            });
        if spawned.is_err() {
            // No helper thread means no answer will ever arrive — report
            // Unavailable instead of parking the flow forever.
            let _ = channel.0.send(DialogAnswer::Unavailable);
        }
    }

    fn poll_answer(&mut self) -> Option<DialogAnswer> {
        let channel = self.answers.as_ref()?;
        match channel.1.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            // The helper thread died without an answer (its panic): the
            // stored sender keeps the channel alive, so this arm is only
            // reachable if the channel was torn down — treat as
            // Unavailable, never as a user's Cancel.
            Err(mpsc::TryRecvError::Disconnected) => Some(DialogAnswer::Unavailable),
        }
    }
}

/// Drives one future to completion on the calling thread without an
/// async runtime: the waker signals a condition variable, so a pending
/// future parks this thread until it is woken from elsewhere. No
/// `unsafe`, no futures crate — `std::task::Wake` is the whole
/// machinery.
fn block_on<F: Future>(future: F) -> F::Output {
    struct Signal(std::sync::Mutex<bool>, std::sync::Condvar);

    impl Wake for Signal {
        fn wake(self: Arc<Self>) {
            let mut woken = self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *woken = true;
            self.1.notify_all();
        }
    }

    let signal = Arc::new(Signal(
        std::sync::Mutex::new(false),
        std::sync::Condvar::new(),
    ));
    let waker = Waker::from(Arc::clone(&signal));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                let mut woken = signal
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*woken {
                    woken = signal
                        .1
                        .wait(woken)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                *woken = false;
            }
        }
    }
}

/// Scripted host for the headless suite: answers are queued per dialog kind
/// and consumed in order; an empty queue answers "cancel"/"no".
#[derive(Debug, Default)]
pub struct ScriptedDialogs {
    files: Vec<Option<PathBuf>>,
    folders: Vec<Option<PathBuf>>,
    saves: Vec<Option<PathBuf>>,
    confirms: Vec<bool>,
    /// The requests the app parked (test pins).
    requests: std::collections::VecDeque<DialogRequest>,
}

impl ScriptedDialogs {
    /// The requests the app parked, in order (test pins).
    pub fn requests(&self) -> &[DialogRequest] {
        self.requests.as_slices().0
    }
}

impl ScriptedDialogs {
    /// Starts a script.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one open-file answer.
    pub fn file(mut self, answer: Option<&str>) -> Self {
        self.files.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one open-folder answer.
    pub fn folder(mut self, answer: Option<&str>) -> Self {
        self.folders.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one save-file answer.
    pub fn save(mut self, answer: Option<&str>) -> Self {
        self.saves.push(answer.map(PathBuf::from));
        self
    }

    /// Queues one confirmation answer.
    pub fn confirm(mut self, answer: bool) -> Self {
        self.confirms.push(answer);
        self
    }
}

impl DialogQueue for ScriptedDialogs {
    /// Records the request; the queued answer is consumed on the next
    /// `poll_answer` — the same queue discipline the native host runs.
    fn request(&mut self, request: DialogRequest) {
        self.requests.push_back(request);
    }

    fn poll_answer(&mut self) -> Option<DialogAnswer> {
        let request = self.requests.pop_front()?;
        Some(match request {
            DialogRequest::OpenFile { .. } => DialogAnswer::Path(if self.files.is_empty() {
                None
            } else {
                self.files.remove(0)
            }),
            DialogRequest::OpenFolder => DialogAnswer::Path(if self.folders.is_empty() {
                None
            } else {
                self.folders.remove(0)
            }),
            DialogRequest::SaveFile { .. } => DialogAnswer::Path(if self.saves.is_empty() {
                None
            } else {
                self.saves.remove(0)
            }),
            DialogRequest::Confirm { .. } => DialogAnswer::Confirmed(if self.confirms.is_empty() {
                false
            } else {
                self.confirms.remove(0)
            }),
        })
    }
}

#[cfg(test)]
mod queue_contract {
    //! The request/answer queue contract (step 81.1, item M97): the seam
    //! parks requests, answers arrive on a later poll, and the scripted
    //! host answers through the same queue the native host will.

    use std::task::{Context, Poll};

    use super::*;

    #[test]
    fn the_scripted_host_answers_requests_through_the_same_queue() {
        let mut host = ScriptedDialogs::new()
            .file(Some("/tmp/a.csv"))
            .confirm(true);
        host.request(DialogRequest::OpenFile { extensions: &[] });
        host.request(DialogRequest::Confirm {
            title: "Delete Object".into(),
            message: "Delete Card? This cannot be undone.".into(),
        });
        assert_eq!(
            host.poll_answer(),
            Some(DialogAnswer::Path(Some(PathBuf::from("/tmp/a.csv"))))
        );
        assert_eq!(host.poll_answer(), Some(DialogAnswer::Confirmed(true)));
        assert_eq!(host.poll_answer(), None, "the queue is drained");
    }

    #[test]
    fn an_unscripted_request_answers_cancel_like_the_old_seam() {
        let mut host = ScriptedDialogs::new();
        host.request(DialogRequest::SaveFile {
            suggested: "database.mdb".into(),
        });
        assert_eq!(host.poll_answer(), Some(DialogAnswer::Path(None)));
        host.request(DialogRequest::Confirm {
            title: "t".into(),
            message: "m".into(),
        });
        assert_eq!(host.poll_answer(), Some(DialogAnswer::Confirmed(false)));
        assert_eq!(host.poll_answer(), None, "no request, no answer");
    }

    #[test]
    fn the_requests_are_recorded_for_test_pins() {
        let mut host = ScriptedDialogs::new();
        host.request(DialogRequest::SaveFile {
            suggested: "database.mdb".into(),
        });
        host.request(DialogRequest::OpenFolder);
        assert_eq!(
            host.requests(),
            &[
                DialogRequest::SaveFile {
                    suggested: "database.mdb".into()
                },
                DialogRequest::OpenFolder,
            ]
        );
    }

    #[test]
    fn the_answer_kinds_stay_distinguishable() {
        assert_ne!(
            DialogAnswer::Path(Some(PathBuf::from("x"))),
            DialogAnswer::Path(None)
        );
        assert_ne!(DialogAnswer::Path(None), DialogAnswer::Unavailable);
        assert_ne!(DialogAnswer::Confirmed(true), DialogAnswer::Unavailable);
    }

    #[test]
    fn the_mini_executor_drives_ready_and_woken_futures() {
        assert_eq!(super::block_on(async { 41 + 1 }), 42);

        /// Goes pending on the first poll, schedules a wake on another
        /// thread, and completes on the second — the condvar executor
        /// must park and re-poll, not spin.
        struct WakeLater(Option<std::task::Waker>);
        impl Future for WakeLater {
            type Output = u8;
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
                match self.0.take() {
                    None => {
                        let waker = cx.waker().clone();
                        let wake = waker.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            wake.wake();
                        });
                        self.0 = Some(waker);
                        Poll::Pending
                    }
                    Some(_) => Poll::Ready(9),
                }
            }
        }
        assert_eq!(super::block_on(WakeLater(None)), 9);
    }
}
