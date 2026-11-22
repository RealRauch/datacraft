//! Compact and the design sidecar (phase 95, step 95.2; item M126): the
//! atomic swap happens at the original path, so the sibling
//! `.datacraft` sidecar stays the database's design authority — the bytes
//! are untouched and a reopen still auto-loads it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::port::{DatabaseFormat, StoragePort};
use datacraft_storage::detecting::DetectingPort;

fn scratch_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-compact-sidecar-{}-{n}.mdb",
        std::process::id()
    ))
}

struct Scratch(Vec<PathBuf>);

impl Drop for Scratch {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[test]
fn compacting_at_the_same_path_keeps_the_sidecar_authority() {
    let path = scratch_path();
    let sidecar = path.with_extension("datacraft");
    let _scratch = Scratch(vec![path.clone(), sidecar.clone()]);
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let created = core.dispatch(Command::CreateDatabase {
        path: path.clone(),
        format: DatabaseFormat::Jet4,
    });
    assert!(
        !created
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{created:?}"
    );
    let opened = core.dispatch(Command::OpenDatabase {
        path: path.clone(),
        password: None,
        source: None,
    });
    assert!(matches!(opened.first(), Some(Event::DatabaseOpened { .. })));
    core.dispatch(Command::CreateReport {
        name: "Weekly".into(),
    });
    let saved = core.dispatch(Command::SaveDesignDocument {
        path: sidecar.clone(),
    });
    assert!(
        !saved
            .iter()
            .any(|event| matches!(event, Event::Failed { .. })),
        "{saved:?}"
    );
    let before = std::fs::read(&sidecar).expect("the sidecar exists");

    let events = core.dispatch(Command::CompactDatabase);
    assert!(
        matches!(events.as_slice(), [Event::DatabaseCompacted { .. }]),
        "{events:?}"
    );
    assert_eq!(
        std::fs::read(&sidecar).expect("the sidecar still exists"),
        before,
        "the swap at the same path leaves the sidecar untouched"
    );

    // A fresh session finds the sidecar by the unchanged path.
    let mut reopened = Core::new(Box::new(DetectingPort::new()));
    let events = reopened.dispatch(Command::OpenDatabase {
        path: path.clone(),
        password: None,
        source: None,
    });
    assert!(matches!(events.first(), Some(Event::DatabaseOpened { .. })));
    let database = reopened.inspect().database.expect("open");
    assert_eq!(database.sidecar_error, None, "the sidecar auto-loads");
    assert_eq!(database.reports.len(), 1, "the report design came back");

    let mut port = DetectingPort::new();
    port.open(&path, None)
        .expect("the compacted file still opens");
    assert_eq!(port.driver_id().as_str(), "jet");
}
