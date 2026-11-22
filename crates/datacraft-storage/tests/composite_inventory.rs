//! The detecting composite forwards the per-source inventory (F13).
//!
//! `DetectingPort` selects one session driver per open; every trait query
//! must delegate to that session. The file-owned inventory (facts, health,
//! native views, indexes, triggers, native Access objects) is part of the
//! same contract — a missing delegation silently empties the snapshot for
//! every real client even though the backend reports it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::StoragePort;
use datacraft_storage::detecting::DetectingPort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-composite-inventory-{}-{n}-{stem}.sqlite",
        std::process::id()
    ))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn the_composite_forwards_facts_and_the_file_inventory() {
    let path = scratch_path("inventory");
    let conn = rusqlite::Connection::open(&path).expect("create scratch database");
    conn.execute_batch(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
         CREATE INDEX items_name ON items (name);
         CREATE VIEW named_items AS SELECT name FROM items;
         CREATE TRIGGER items_touch AFTER INSERT ON items BEGIN
             UPDATE items SET name = name WHERE id = NEW.id;
         END;",
    )
    .expect("schema");
    drop(conn);
    let scratch = Scratch(path.clone());

    let mut port = DetectingPort::new();
    port.open(&scratch.0, None).expect("composite opens SQLite");

    let facts = port.source_facts();
    assert!(
        facts.facts.iter().any(|fact| fact.name == "sqlite_version"),
        "facts must survive the composite: {:?}",
        facts.facts
    );
    assert!(port.native_views().expect("views").len() == 1);
    assert!(port.indexes().expect("indexes").len() >= 1);
    assert_eq!(port.triggers().expect("triggers").len(), 1);
    assert!(port.native_objects().expect("native objects").is_empty());
}
