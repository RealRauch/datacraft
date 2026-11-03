//! Jet4 writer-level row update contracts (step 11.10, item H40).
//!
//! These tests drive `Jet4Writer` directly to reach the paths a port test
//! cannot address (in-place writes, page repacking, overflow pointers and
//! the raw AutoNumber seed). Fixture copies only — fixtures are immutable
//! evidence (docs/TESTING.md §4).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_jet::StorageError;
use datacraft_jet::StorageValue;
use datacraft_jet::jet::{DataPage, Jet4Writer, JetColumn, JetType};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// A private copy of the pinned fixture (never write the original).
fn scratch_copy(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "datacraft-jet-writer-rows-{}-{n}-{stem}.mdb",
        std::process::id()
    ));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn writer_row_updates_cover_in_place_external_and_full_pages() {
    let scratch = Scratch(scratch_copy("writer-rows"));
    let bytes = std::fs::read(&scratch.0).expect("read fixture");
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    let page = writer
        .create_table(
            "RowOps",
            &[
                JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("note", JetType::Memo, 0x02, 1, 0, 0),
            ],
        )
        .expect("create the table");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("abcde".into())],
        )
        .expect("insert the row");

    // Same-length text: the writer overwrites the slot in place.
    writer
        .update_row(
            page,
            0,
            &[StorageValue::Int(1), StorageValue::Text("vwxyz".into())],
        )
        .expect("in-place update");

    // A row that no longer fits the page answers a precise Unsupported.
    let huge = "x".repeat(5000);
    let err = writer
        .update_row(page, 0, &[StorageValue::Int(1), StorageValue::Text(huge)])
        .expect_err("row larger than the page");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("does not fit"), "{err}");

    // A row that fits the page but collides with the offset table too.
    let tight = "x".repeat(2035);
    let err = writer
        .update_row(page, 0, &[StorageValue::Int(1), StorageValue::Text(tight)])
        .expect_err("row collides with the offset table");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");

    // Patch the memo header into the external LVAL form: the update must be
    // refused instead of orphaning the chain.
    let mut bytes = writer.into_file();
    let (page_no, slot) = Jet4Writer::new(bytes.clone())
        .expect("writer")
        .row_location(page, 0)
        .expect("locate the row");
    let base = page_no as usize * 4096;
    let decoded = DataPage::decode(&bytes[base..base + 4096]).expect("decode the page");
    let (start, _, _) = decoded.records[slot];
    let memo_at = base + start + 2 + 4; // num_cols + fixed area
    bytes[memo_at..memo_at + 4].copy_from_slice(&0x4000_0000u32.to_le_bytes());
    let mut patched = Jet4Writer::new(bytes).expect("writer");
    let err = patched
        .update_row(
            page,
            0,
            &[StorageValue::Int(1), StorageValue::Text("new".into())],
        )
        .expect_err("external LVAL row");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("LVAL"), "{err}");
}

#[test]
fn rows_behind_an_overflow_pointer_are_not_updated() {
    let scratch = Scratch(scratch_copy("overflow"));
    let bytes = std::fs::read(&scratch.0).expect("read fixture");
    let mut writer = Jet4Writer::new(bytes).expect("writer");
    let page = writer
        .create_table(
            "Overflow",
            &[
                JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("note", JetType::Memo, 0x02, 1, 0, 0),
            ],
        )
        .expect("create the table");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(1), StorageValue::Text("first".into())],
        )
        .expect("insert the data row");
    writer
        .insert_row(
            page,
            &[StorageValue::Int(2), StorageValue::Text("second".into())],
        )
        .expect("insert the pointer row");

    // Rewire slot 1 into an overflow source that points at slot 0 (the
    // target), exactly as Access stores a row larger than one page.
    let mut bytes = writer.into_file();
    let (page_no, _) = Jet4Writer::new(bytes.clone())
        .expect("writer")
        .row_location(page, 0)
        .expect("locate the first row");
    let base = page_no as usize * 4096;
    let decoded = DataPage::decode(&bytes[base..base + 4096]).expect("decode the page");
    let (_, _, raw0) = decoded.records[0];
    let (start1, _, raw1) = decoded.records[1];
    let pointer = (page_no << 8).to_le_bytes();
    bytes[base + start1..base + start1 + 4].copy_from_slice(&pointer);
    bytes[base + 14 + 2..base + 16 + 2].copy_from_slice(&(raw1 | 0x4000).to_le_bytes());
    bytes[base + 14..base + 16].copy_from_slice(&(raw0 | 0x8000).to_le_bytes());

    let mut patched = Jet4Writer::new(bytes).expect("writer");
    let err = patched
        .update_row(
            page,
            0,
            &[StorageValue::Int(9), StorageValue::Text("new".into())],
        )
        .expect_err("overflow row");
    assert!(matches!(err, StorageError::Unsupported { .. }), "{err:?}");
    assert!(err.to_string().contains("overflow"), "{err}");
}
