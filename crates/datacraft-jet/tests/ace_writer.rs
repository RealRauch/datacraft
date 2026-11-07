//! ACCDB write contracts (phase 25, step 25.1, item H42).
//!
//! The Jet4 writer proves its contracts on byte copies of the pinned ACE
//! fixtures `testV2007.accdb` / `testV2010.accdb`. There is no external ACE
//! oracle (mdb-tools reads no `.accdb`) — the reference is
//! `decode ∘ op ∘ decode` against the pristine bytes, so the CI oracle job
//! does not extend. Contracts: every original page stays byte-identical
//! except the pages a write legitimately touches in place; usage maps stay
//! consistent with the page scan; unqualified shapes answer `Unsupported`.

use datacraft_core::port::SchemaChange;
use datacraft_core::schema::{FieldDef, FieldKind};
use datacraft_jet::StorageError;
use datacraft_jet::StorageValue;
use datacraft_jet::jet::complex::ComplexCatalog;
use datacraft_jet::jet::{
    CATALOG_TDEF_PAGE, Jet4TableReader, Jet4Writer, JetColumn, JetType, OBJECT_TYPE_TABLE,
};

mod support;

/// ACE page size (format-derived, same as Jet4).
const PAGE: usize = 4096;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name);
    std::fs::read(path).unwrap_or_else(|e| panic!("fixture {name} is pinned: {e}"))
}

/// The write-contract fixtures: ACE 2007 and ACE 2010.
fn write_fixtures() -> [&'static str; 2] {
    ["testV2007.accdb", "testV2010.accdb"]
}

/// `(name, tdef_page)` of every user-visible table (catalog order). Hidden
/// complex-value backing tables (`f_<guid>_Data`) are excluded exactly like
/// the port hides them — they are implementation details, not user tables.
fn user_tables(reader: &Jet4TableReader) -> Vec<(String, u32)> {
    let complex = ComplexCatalog::read(reader).expect("complex catalog decodes");
    reader
        .catalog()
        .expect("catalog decodes")
        .into_iter()
        .filter(|entry| {
            entry.object_type == OBJECT_TYPE_TABLE
                && !entry.name.starts_with("MSys")
                && entry
                    .tdef_page
                    .is_some_and(|page| !complex.is_flat_page(page))
        })
        .map(|entry| (entry.name, entry.tdef_page.expect("checked above")))
        .collect()
}

/// TDEF page of the fixture's `Table1`.
fn table1_tdef(reader: &Jet4TableReader) -> u32 {
    user_tables(reader)
        .into_iter()
        .find(|(name, _)| name == "Table1")
        .expect("the fixture holds Table1")
        .1
}

/// Page numbers (ascending, original range only) whose bytes differ.
fn changed_pages(before: &[u8], after: &[u8]) -> Vec<u32> {
    (0..before.len() / PAGE)
        .filter(|&p| before[p * PAGE..(p + 1) * PAGE] != after[p * PAGE..(p + 1) * PAGE])
        .map(|p| p as u32)
        .collect()
}

/// Exactly `expected` pages (order-insensitive) may differ — append-only
/// writes never touch anything outside the legitimate in-place set.
fn assert_changed_exactly(before: &[u8], after: &[u8], mut expected: Vec<u32>, context: &str) {
    expected.sort_unstable();
    let mut changed = changed_pages(before, after);
    changed.sort_unstable();
    assert_eq!(changed, expected, "{context}: unexpected pages changed");
}

/// Every page changed by `after` must be one of `allowed`.
fn assert_only_these_changed(before: &[u8], after: &[u8], allowed: &[u32], context: &str) {
    let unexpected: Vec<u32> = changed_pages(before, after)
        .into_iter()
        .filter(|page| !allowed.contains(page))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{context}: pages {unexpected:?} changed, allowed {allowed:?}"
    );
}

/// Usage maps must agree with the page scan: no page is both free-marked in
/// the global map and owned by a live table's used map (real files carry
/// stale free bits and pages freed by drops — both are fine as long as no
/// live page is claimed free), every used map owns its data pages and marks
/// nothing beyond EOF.
fn assert_maps_consistent(writer: &Jet4Writer) {
    let page_count = writer.header().page_count as u32;
    let global = writer.global_free_map().expect("global free map");
    for tdef in writer.tdef_pages() {
        let (used_ref, used) = writer.table_used_map(tdef).expect("used map decodes");
        for page in used.marked_pages() {
            assert!(
                page < page_count,
                "used map {tdef} marks page {page} beyond EOF"
            );
            assert!(
                !global.owns_free(page),
                "page {page} owned by tdef {tdef} (map at {}) but free-marked",
                used_ref.page
            );
        }
        for data in writer.data_pages_of(tdef) {
            assert!(
                used.owns(data),
                "data page {data} of tdef {tdef} is unmarked"
            );
        }
    }
}

/// The pages a catalog mutation (create/drop) may legitimately touch in
/// place: global free map, catalog TDEF, its usage maps and data pages.
fn catalog_page_set(writer: &Jet4Writer) -> Vec<u32> {
    let (used, _) = writer
        .table_used_map(CATALOG_TDEF_PAGE)
        .expect("catalog used map");
    let (free, _) = writer
        .table_free_map(CATALOG_TDEF_PAGE)
        .expect("catalog free map");
    let mut pages = vec![1u32, CATALOG_TDEF_PAGE, used.page, free.page];
    pages.extend(writer.data_pages_of(CATALOG_TDEF_PAGE));
    // The catalog's physical index leaves stay in sync with catalog rows
    // (step 49.2, item M63): creates and drops rewrite them.
    pages.extend(support::index_leaf_pages(writer, CATALOG_TDEF_PAGE));
    pages
}

/// A row template with both text cells replaced (schema-agnostic: every
/// other value is carried over unchanged).
fn rewritten_row(template: &[StorageValue], first: &str, second: &str) -> Vec<StorageValue> {
    let replacements = [first, second];
    let mut texts = replacements.iter();
    template
        .iter()
        .map(|value| match value {
            StorageValue::Text(_) => {
                StorageValue::Text(texts.next().copied().unwrap_or("").to_owned())
            }
            other => other.clone(),
        })
        .collect()
}

/// Office-encrypted ACCDB files stay unqualified at the writer boundary —
/// precisely like the reader boundary (M5).
#[test]
fn an_office_encrypted_accdb_is_rejected_at_the_writer_boundary() {
    for name in ["passwordV2010.accdb", "officeAgile42.accdb"] {
        let error = match Jet4Writer::new(fixture(name)) {
            Err(error) => error,
            Ok(_) => panic!("{name}: an office-encrypted ACCDB must not open for writes"),
        };
        match error {
            StorageError::Unsupported { reason } => {
                assert!(reason.contains("Office encryption"), "{name}: {reason}");
            }
            other => panic!("{name}: expected Unsupported, got {other:?}"),
        }
    }
}

/// Inserting into a real ACE table extends its data page in place and bumps
/// the row count — every other original page is byte-identical.
#[test]
fn insert_row_extends_an_ace_table_and_preserves_every_other_page() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let tdef = table1_tdef(&reader);
        let template = reader.read_table(tdef).expect("rows decode").rows.remove(0);
        let row = rewritten_row(&template, "writer", "ace");

        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        let mut expected: Vec<u32> = writer.data_pages_of(tdef).collect();
        expected.push(tdef);
        expected.extend(support::index_leaf_pages(&writer, tdef));
        writer
            .insert_row(tdef, &row)
            .expect("insert into the ACE table");
        let after = writer.into_file();

        assert_changed_exactly(&before, &after, expected, name);
        assert_maps_consistent(&Jet4Writer::new(after.clone()).expect("reopens"));
        let reopened = Jet4TableReader::new(after).expect("written file decodes");
        let rows = reopened.read_table(tdef).expect("table reads").rows;
        assert_eq!(rows.len(), 3, "{name}: the insert persists");
        assert_eq!(rows[2], row, "{name}: the appended row decodes");
    }
}

/// Updating a real ACE row rewrites the data page (repack on growth) and
/// nothing else; the neighbouring row keeps its values.
#[test]
fn update_row_replaces_an_ace_fixture_row_in_place() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let tdef = table1_tdef(&reader);
        let rows = reader.read_table(tdef).expect("rows decode").rows;
        let large = rows[1].clone();

        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        let mut allowed: Vec<u32> = writer.data_pages_of(tdef).collect();
        allowed.push(tdef);
        allowed.extend(support::index_leaf_pages(&writer, tdef));
        writer
            .update_row(tdef, 0, &large)
            .expect("update overwrites the first row");
        let after = writer.into_file();

        // The key image and the index statistics record change with the new
        // value; nothing outside the table's own pages may move.
        assert_only_these_changed(&before, &after, &allowed, name);
        let reopened = Jet4TableReader::new(after).expect("written file decodes");
        let rows = reopened.read_table(tdef).expect("table reads").rows;
        assert_eq!(rows.len(), 2, "{name}");
        assert_eq!(rows[0], large, "{name}: the update persists");
        assert_eq!(rows[1], large, "{name}: the neighbour is untouched");
    }
}

/// Deleting a real ACE row flags the slot (bytes stay in place) and
/// decrements the row count — data page plus TDEF, nothing else.
#[test]
fn delete_row_marks_an_ace_row_deleted() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let tdef = table1_tdef(&reader);
        let survivor = reader.read_table(tdef).expect("rows decode").rows.remove(1);

        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        let mut expected: Vec<u32> = writer.data_pages_of(tdef).collect();
        expected.push(tdef);
        expected.extend(support::index_leaf_pages(&writer, tdef));
        writer.delete_row(tdef, 0).expect("delete the first row");
        let after = writer.into_file();

        assert_changed_exactly(&before, &after, expected, name);
        let reopened = Jet4TableReader::new(after).expect("written file decodes");
        let rows = reopened.read_table(tdef).expect("table reads").rows;
        assert_eq!(rows, [survivor], "{name}: exactly the second row survives");
    }
}

/// Creating a table on an ACE file appends usage/TDEF/data pages and a
/// catalog row; every original page outside the catalog set is untouched.
#[test]
fn create_table_appends_and_catalogues_on_ace_files() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let baseline = user_tables(&reader);
        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        let allowed = catalog_page_set(&writer);

        let columns = vec![
            JetColumn::new("id", JetType::LongInt, 0x03, 0, 0, 4),
            JetColumn::new("name", JetType::Text, 0x02, 1, 0, 80),
            JetColumn::new("active", JetType::Bool, 0x01, 2, 0, 0),
        ];
        let people_tdef = writer
            .create_table("People", &columns)
            .expect("create on ACE");
        let row = [
            StorageValue::Int(7),
            StorageValue::Text("Ada".into()),
            StorageValue::Bit(true),
        ];
        writer
            .insert_row(people_tdef, &row)
            .expect("insert into the new table");
        let after = writer.into_file();

        assert_eq!(
            after.len() - before.len(),
            3 * PAGE,
            "{name}: three pages appended"
        );
        assert_only_these_changed(&before, &after, &allowed, name);
        // The three appended pages are allocated in the global free map.
        let appended_writer = Jet4Writer::new(after.clone()).expect("reopens");
        let global = appended_writer.global_free_map().expect("global free map");
        for page in (before.len() / PAGE) as u32..(after.len() / PAGE) as u32 {
            assert!(
                !global.owns_free(page),
                "{name}: appended page {page} is unallocated"
            );
        }
        assert_maps_consistent(&appended_writer);

        let reopened = Jet4TableReader::new(after).expect("written file decodes");
        let mut tables = user_tables(&reopened);
        assert_eq!(
            tables.len(),
            baseline.len() + 1,
            "{name}: People is catalogued"
        );
        let people = tables
            .drain(..)
            .find(|(table, _)| table == "People")
            .expect("People");
        assert_eq!(
            people.1, people_tdef,
            "{name}: the TDEF page is the identity"
        );
        let rows = reopened.read_table(people.1).expect("People reads").rows;
        assert_eq!(rows, [row], "{name}: the row persists");
    }
}

/// Dropping a real fixture table frees exactly its pages (0xFF fill), keeps
/// the catalog consistent and preserves every other page.
#[test]
fn drop_table_removes_a_real_ace_table_and_frees_its_pages() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        let mut allowed = catalog_page_set(&writer);

        let tdef = table1_tdef(&reader);
        let mut freed = vec![tdef];
        freed.extend(writer.data_pages_of(tdef));
        freed.push(writer.table_used_map(tdef).expect("used map").0.page);
        freed.push(writer.table_free_map(tdef).expect("free map").0.page);
        allowed.extend(freed.iter().copied());

        writer.drop_table(tdef).expect("drop the fixture table");
        let after = writer.into_file();

        assert_only_these_changed(&before, &after, &allowed, name);
        // Every page the dropped table owned carries the documented freed
        // pattern; the catalog's pages keep their content.
        for page in &freed {
            let base = *page as usize * PAGE;
            assert!(
                after[base..base + PAGE].iter().all(|&byte| byte == 0xFF),
                "{name}: freed page {page} must be 0xFF-filled"
            );
        }
        assert_maps_consistent(&Jet4Writer::new(after.clone()).expect("reopens"));

        let reopened = Jet4TableReader::new(after).expect("written file decodes");
        assert!(
            !user_tables(&reopened)
                .iter()
                .any(|(candidate, _)| candidate == "Table1"),
            "{name}: Table1 is gone"
        );
        reopened
            .read_table(CATALOG_TDEF_PAGE)
            .expect("the catalog itself still reads");
    }
}

/// ALTER TABLE on a real ACE table: an appended nullable field fills Null
/// into existing rows, a rename keeps row bytes stable (TDEF-only change).
#[test]
fn alter_table_extends_and_renames_on_ace_tables() {
    for name in write_fixtures() {
        let before = fixture(name);
        let reader = Jet4TableReader::new(before.clone()).expect("pristine file decodes");
        let tdef = table1_tdef(&reader);
        let data = reader.read_table(tdef).expect("rows decode");
        let first_column = data.schema.columns[0].name.clone();
        let old_column_count = data.schema.columns.len();
        let old_row_count = data.rows.len();
        let data_pages: Vec<u32> = {
            let writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
            writer.data_pages_of(tdef).collect()
        };

        // 1. Add a nullable field: rows gain a trailing Null.
        let add = SchemaChange::AddField {
            field: FieldDef::new("extra", FieldKind::Text { max_len: 20 }),
        };
        let mut writer = Jet4Writer::new(before.clone()).expect("writer opens ACE");
        writer.alter_table(tdef, &add).expect("add field");
        let after_add = writer.into_file();
        let mut allowed = data_pages.clone();
        allowed.push(tdef);
        assert_only_these_changed(&before, &after_add, &allowed, name);
        let reopened = Jet4TableReader::new(after_add.clone()).expect("written file decodes");
        let data = reopened.read_table(tdef).expect("table reads");
        assert_eq!(data.schema.columns.len(), old_column_count + 1, "{name}");
        assert_eq!(
            data.schema.columns.last().expect("appended").name,
            "extra",
            "{name}"
        );
        for row in &data.rows {
            assert_eq!(
                row.last(),
                Some(&StorageValue::Null),
                "{name}: rows gain Null"
            );
        }
        assert_eq!(data.rows.len(), old_row_count, "{name}");

        // 2. Rename the first field on the written buffer (its header region
        // is still the raw on-disk form — no reader round-trip): TDEF only.
        let rename = SchemaChange::RenameField {
            field: first_column,
            new_name: "renamed_field".to_owned(),
        };
        let mut writer = Jet4Writer::new(after_add.clone()).expect("reopens");
        writer.alter_table(tdef, &rename).expect("rename field");
        let after_rename = writer.into_file();
        assert_eq!(
            changed_pages(&after_add, &after_rename),
            [tdef],
            "{name}: TDEF only"
        );
        let reopened = Jet4TableReader::new(after_rename).expect("written file decodes");
        let schema = reopened.read_table(tdef).expect("table reads").schema;
        assert_eq!(schema.columns[0].name, "renamed_field", "{name}");
    }
}

/// Unqualified ALTER shapes answer a precise Unsupported on ACE too and
/// leave the file untouched.
#[test]
fn unsupported_alters_stay_precise_on_ace_and_touch_nothing() {
    for name in write_fixtures() {
        let file = fixture(name);
        let reader = Jet4TableReader::new(file.clone()).expect("pristine file decodes");
        let tdef = table1_tdef(&reader);
        let mut writer = Jet4Writer::new(file).expect("writer opens ACE");
        let snapshot = writer.file().to_vec();

        let mut required = FieldDef::new("strict", FieldKind::Text { max_len: 5 });
        required.properties.required = true;
        let error = writer
            .alter_table(tdef, &SchemaChange::AddField { field: required })
            .expect_err("required fields need a default for existing rows");
        assert!(
            matches!(error, StorageError::Unsupported { .. }),
            "{name}: {error:?}"
        );

        assert_eq!(
            writer.file(),
            &snapshot,
            "{name}: rejected alters touch nothing"
        );
    }
}

/// The 2016 fixture is the read-only reference: the writer opens it but a
/// no-op pass leaves every byte — and every decoded row — unchanged.
#[test]
fn the_2016_fixture_is_a_stable_read_only_reference() {
    let file = fixture("attachmentsV2016.accdb");
    let before = Jet4TableReader::new(file.clone()).expect("pristine file decodes");
    let snapshot: Vec<(u32, Vec<Vec<StorageValue>>)> = user_tables(&before)
        .into_iter()
        .map(|(_, tdef)| {
            let rows = before.read_table(tdef).expect("rows decode").rows;
            (tdef, rows)
        })
        .collect();
    assert!(!snapshot.is_empty(), "the 2016 fixture carries user tables");

    let writer = Jet4Writer::new(file.clone()).expect("the writer opens ACE 2016");
    let after = writer.into_file();
    assert_eq!(after, file, "a no-op writer pass must not modify the file");

    let reopened = Jet4TableReader::new(after).expect("written file decodes");
    for (tdef, rows) in snapshot {
        assert_eq!(
            reopened.read_table(tdef).expect("table reads").rows,
            rows,
            "tdef {tdef}: rows unchanged"
        );
    }
}
