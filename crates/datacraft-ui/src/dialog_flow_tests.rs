//! The parked dialog flows' contract tests (step 81.2, item M97) —
//! split from `app_tests.rs` for the file-size gate.

//! The parked dialog flows (step 81.2, item M97): a request parks,
//! the per-frame poll runs its intent, Cancel/Unavailable never
//! mutate state, and only one dialog is pending at a time.

use crate::app::App;
use crate::app::dialog_flow::DialogIntent;
use crate::dialogs::{DialogAnswer, DialogQueue, DialogRequest, ScriptedDialogs};

/// A host that answers nothing at all — the Unavailable branch.
struct DeadHost;

impl DialogQueue for DeadHost {
    fn request(&mut self, _request: DialogRequest) {}

    fn poll_answer(&mut self) -> Option<DialogAnswer> {
        Some(DialogAnswer::Unavailable)
    }
}

#[test]
fn a_parked_dialog_intent_runs_on_a_later_poll() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_dialogs(Box::new(
        ScriptedDialogs::new().file(Some("C:/data/shop.mdb")),
    ));
    app.request_dialog(DialogRequest::OpenFile, DialogIntent::FillOpenPath);
    assert!(
        app.open_path.is_empty(),
        "the answer arrives on a later frame, never inside the click"
    );
    app.poll_dialogs();
    assert_eq!(app.open_path, "C:/data/shop.mdb");
    app.poll_dialogs();
    assert_eq!(app.open_path, "C:/data/shop.mdb", "the park is drained");
}

#[test]
fn a_cancelled_dialog_clears_the_park_without_running_the_intent() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_dialogs(Box::new(ScriptedDialogs::new()));
    app.request_dialog(DialogRequest::OpenFile, DialogIntent::FillOpenPath);
    app.poll_dialogs();
    assert!(
        app.open_path.is_empty(),
        "a Cancel keeps every buffer untouched"
    );
    assert!(
        !app.has_pending_dialog(),
        "the park is cleared so the flow can be retried"
    );
}

#[test]
fn a_second_request_while_one_is_pending_is_ignored() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    let host = ScriptedDialogs::new()
        .file(Some("C:/data/first.mdb"))
        .file(Some("C:/data/second.mdb"));
    app.set_dialogs(Box::new(host));
    app.request_dialog(DialogRequest::OpenFile, DialogIntent::FillOpenPath);
    app.request_dialog(DialogRequest::OpenFile, DialogIntent::FillOpenPath);
    app.poll_dialogs();
    assert_eq!(app.open_path, "C:/data/first.mdb");
    app.poll_dialogs();
    assert_eq!(
        app.open_path, "C:/data/first.mdb",
        "only one request was ever parked"
    );
}

#[test]
fn an_unavailable_dialog_answers_a_precise_notice() {
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_dialogs(Box::new(DeadHost));
    app.request_dialog(DialogRequest::OpenFile, DialogIntent::FillOpenPath);
    app.poll_dialogs();
    assert!(
        !app.has_pending_dialog(),
        "the park is cleared after the precise answer"
    );
    let error = app
        .last_error()
        .expect("the unavailable host is an error, not a silent Cancel");
    assert!(
        error.contains("dialog"),
        "the notice names the dialog problem: {error}"
    );
}

#[test]
fn the_delete_flow_parks_and_dispatches_on_the_confirmation() {
    use datacraft_core::ObjectKind;
    let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
    app.set_dialogs(Box::new(ScriptedDialogs::new().confirm(true)));
    app.dispatch(datacraft_core::command::Command::OpenDatabase {
        path: "memory.mock".into(),
        password: None,
        source: None,
    });
    app.dispatch(datacraft_core::command::Command::CreateForm {
        name: "Card".into(),
    });
    assert!(!app.has_pending_dialog());
    app.delete_object(ObjectKind::Form, "Card");
    assert!(
        app.has_pending_dialog(),
        "the confirmation parks before it dispatches"
    );
    assert!(
        app.snapshot()
            .database
            .as_ref()
            .expect("database")
            .forms
            .iter()
            .any(|form| form.name == "Card"),
        "nothing is deleted before the answer"
    );
    app.poll_dialogs();
    assert!(
        !app.snapshot()
            .database
            .as_ref()
            .expect("database")
            .forms
            .iter()
            .any(|form| form.name == "Card"),
        "the confirmed intent deletes on the later frame"
    );
}
