//! Jet4 write-through contract (step 11.1, item H9/H40 groundwork).
//!
//! Every test writes a **copy** of the pinned fixture (fixtures are immutable
//! evidence, docs/TESTING.md §4) and proves the change through our decoder
//! after a reopen. Mapping decisions and their lossiness are documented in
//! `src/port_write.rs` and docs/FORMAT-JET.md.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_jet::port_adapter::Jet4Port;

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
        "datacraft-jet-write-{}-{n}-{stem}.mdb",
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

fn opened(path: &Path) -> Jet4Port {
    let mut port = Jet4Port::new();
    port.open(path, None).expect("open the fixture copy");
    port
}

fn table1_values() -> Vec<FieldValue> {
    vec![
        FieldValue::Text("new".into()),
        FieldValue::Text("row".into()),
        FieldValue::Integer(1),
        FieldValue::Integer(2),
        FieldValue::Integer(3),
        FieldValue::Float(4.5),
        FieldValue::DateTime(45_000.0),
        FieldValue::Currency(12_500),
        FieldValue::Boolean(true),
    ]
}

#[test]
fn insert_row_appends_and_persists_across_a_reopen() {
    let scratch = Scratch(scratch_copy("insert"));
    let mut port = opened(&scratch.0);
    let before = port
        .read_rows("Table1", RowRange { start: 0, len: 100 })
        .expect("read before")
        .len();
    port.insert_row("Table1", &table1_values())
        .expect("insert into the fixture table");
    let after = port
        .read_rows("Table1", RowRange { start: 0, len: 100 })
        .expect("read after");
    assert_eq!(after.len(), before + 1);
    assert_eq!(after[before][0], FieldValue::Text("new".into()));
    port.close();

    let reopened = opened(&scratch.0);
    let rows = reopened
        .read_rows("Table1", RowRange { start: 0, len: 100 })
        .expect("reopen reads the inserted row");
    assert_eq!(rows.len(), before + 1);
    assert_eq!(rows[before][5], FieldValue::Float(4.5));
    assert_eq!(rows[before][7], FieldValue::Currency(12_500));
}

fn people() -> TableDef {
    let mut name = FieldDef::new("name", FieldKind::Text { max_len: 40 });
    name.properties.required = true;
    TableDef::new(
        "People",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            name,
            FieldDef::new("notes", FieldKind::Memo),
            FieldDef::new("photo", FieldKind::Binary(BinaryKind::OleObject)),
            FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

#[test]
fn create_table_and_insert_through_the_port_persist() {
    let scratch = Scratch(scratch_copy("create"));
    let mut port = opened(&scratch.0);
    port.create_table(&people()).expect("create People");
    assert!(port.tables().iter().any(|t| t.name == "People"));

    port.insert_row(
        "People",
        &[
            FieldValue::Integer(7),
            FieldValue::Text("Ada".into()),
            FieldValue::Text("a long note".into()),
            FieldValue::Binary(vec![9, 8, 7]),
            FieldValue::Float(2.5),
            FieldValue::Boolean(true),
        ],
    )
    .expect("insert a row with memo and OLE");
    port.close();

    let reopened = opened(&scratch.0);
    let rows = reopened
        .read_rows("People", RowRange { start: 0, len: 10 })
        .expect("read back");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
    assert_eq!(rows[0][2], FieldValue::Text("a long note".into()));
    assert_eq!(rows[0][3], FieldValue::Binary(vec![9, 8, 7]));
    assert_eq!(rows[0][4], FieldValue::Float(2.5));
    assert_eq!(rows[0][5], FieldValue::Boolean(true));
}

#[test]
fn create_table_rejects_forms_the_writer_cannot_encode() {
    let scratch = Scratch(scratch_copy("unsupported"));
    let mut port = opened(&scratch.0);

    let auto = TableDef::new(
        "Auto",
        vec![FieldDef::new(
            "id",
            FieldKind::AutoNumber(AutoKind::ReplicationId),
        )],
    );
    let err = port
        .create_table(&auto)
        .expect_err("GUID AutoNumbers stay unsupported");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");

    let attachment = TableDef::new(
        "Attach",
        vec![FieldDef::new(
            "data",
            FieldKind::Binary(BinaryKind::Attachment),
        )],
    );
    let err = port
        .create_table(&attachment)
        .expect_err("attachments are a complex-value field");
    assert!(matches!(err, PortError::Unsupported { .. }), "{err:?}");
}

#[test]
fn insert_into_an_unknown_table_is_reported() {
    let scratch = Scratch(scratch_copy("missing"));
    let mut port = opened(&scratch.0);
    let err = port
        .insert_row("Missing", &[FieldValue::Null])
        .expect_err("unknown table");
    assert!(matches!(err, PortError::TableNotFound(_)), "{err:?}");
}

#[test]
fn writes_to_a_read_only_copy_fail_precisely_and_leave_it_intact() {
    let scratch = Scratch(scratch_copy("readonly"));
    let mut port = opened(&scratch.0);
    let before = std::fs::read(&scratch.0).expect("read copy");
    let mut permissions = std::fs::metadata(&scratch.0)
        .expect("metadata")
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&scratch.0, permissions).expect("make read-only");

    let err = port
        .insert_row("Table1", &table1_values())
        .expect_err("read-only files reject writes");
    assert!(matches!(err, PortError::WriteFailed { .. }), "{err:?}");
    let after = std::fs::read(&scratch.0).expect("read copy again");
    assert_eq!(before, after, "a rejected write must not touch the file");

    let mut permissions = std::fs::metadata(&scratch.0)
        .expect("metadata")
        .permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&scratch.0, permissions).expect("restore");
}
