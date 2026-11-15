//! Constraint groupings of SQLite foreign keys (step 87.1, item M103 —
//! split from `sqlite_relationships.rs` for the file-size gate): a
//! composite foreign key reads back as one group carrying both column
//! pairs — the follow resolution's composite pairing input. SQLite
//! names no constraints, so the group name stays `None`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_sqlite::SqlitePort;

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    std::env::temp_dir().join(format!(
        "datacraft-sqlite-relgrp-{}-{n}-{stem}-{stamp}.sqlite",
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
fn composite_foreign_keys_share_one_constraint_group() {
    let path = scratch_path("composite");
    let _scratch = Scratch(path.clone());
    let conn = rusqlite::Connection::open(&path).expect("open scratch");
    conn.execute_batch(
        "CREATE TABLE parents (x INTEGER, y INTEGER, PRIMARY KEY (x, y));
         CREATE TABLE children (a INTEGER, b INTEGER,
             FOREIGN KEY (a, b) REFERENCES parents (x, y));",
    )
    .expect("composite fixture schema");
    drop(conn);
    let mut core = Core::new(Box::new(SqlitePort::new()));
    core.dispatch(Command::OpenDatabase {
        path: path.clone(),
        password: None,
        source: None,
    });
    let database = core.inspect().database.expect("open");
    assert_eq!(
        database.relationship_groups,
        Some(vec![datacraft_core::port::RelationshipGroup {
            name: None,
            table: "children".to_owned(),
            related_table: "parents".to_owned(),
            pairs: vec![
                ("a".to_owned(), "x".to_owned()),
                ("b".to_owned(), "y".to_owned()),
            ],
        }]),
        "the two column pairs share one constraint group in seq order"
    );
}
