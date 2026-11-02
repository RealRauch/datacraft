//! ACCDB port write-through contract (phase 25, step 25.2, item H43).
//!
//! Every write verb the Jet4 port exposes — insert, update, delete, create,
//! alter, drop — runs through to a private copy of the pinned
//! `testV2010.accdb` and survives a reopen, and `DatabaseFormat::Ace` stays
//! stable across every operation. There is no format gate on the write
//! path: ACE rides the same Jet4 writer contracts proven in step 25.1.
//! That is a deliberate, pinned fact — nothing is silently passed through,
//! because there is no guard to open. Fixtures are immutable evidence
//! (docs/TESTING.md §4): every test writes a temp copy.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{DatabaseFormat, PortError, RowRange, SchemaChange, StoragePort};
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
};
use datacraft_storage::port_adapter::Jet4Port;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2010.accdb")
}

/// A private copy of the pinned fixture (never write the original).
fn scratch_copy(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "datacraft-ace-port-{}-{n}-{stem}.accdb",
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

/// Opens a copy and pins the format contract: the port reports ACE.
fn opened(path: &Path) -> Jet4Port {
    let mut port = Jet4Port::new();
    let opened = port.open(path, None).expect("open the ACE copy");
    assert_eq!(
        opened.format,
        DatabaseFormat::Ace,
        "the port must report ACE"
    );
    port
}

fn all_rows() -> RowRange {
    RowRange {
        start: 0,
        len: usize::MAX,
    }
}

/// A full row for the fixture's `Table1`, keyed off the port's own field
/// definitions (schema-agnostic: one value per field kind).
fn table1_row(definition: &TableDef, tag: &str) -> Vec<FieldValue> {
    definition
        .fields
        .iter()
        .map(|field| match &field.kind {
            FieldKind::Text { .. } => FieldValue::Text(tag.to_owned()),
            FieldKind::Number(
                NumericKind::Byte | NumericKind::Integer | NumericKind::LongInteger,
            ) => FieldValue::Integer(3),
            FieldKind::Number(NumericKind::Single | NumericKind::Double) => FieldValue::Float(1.5),
            FieldKind::DateTime => FieldValue::DateTime(45_000.0),
            FieldKind::Currency => FieldValue::Currency(9_500),
            FieldKind::Boolean { .. } => FieldValue::Boolean(true),
            other => panic!("Table1 carries an unexpected kind: {other:?}"),
        })
        .collect()
}

/// The lifecycle table created by [`create_alter_and_drop_round_trip_on_an_ace_copy`].
fn people() -> TableDef {
    TableDef::new(
        "People",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 40 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
        ],
    )
}

#[test]
fn ace_row_writes_persist_across_a_reopen_and_stay_ace() {
    let scratch = Scratch(scratch_copy("rows"));
    let mut port = opened(&scratch.0);
    let definition = port
        .tables()
        .into_iter()
        .find(|table| table.name == "Table1")
        .expect("the fixture holds Table1");
    let before = port.read_rows("Table1", all_rows()).expect("read before");
    assert_eq!(before.len(), 2, "the fixture's Table1 row count");

    port.insert_row("Table1", &table1_row(&definition, "writer"))
        .expect("insert");
    port.update_row("Table1", 0, &table1_row(&definition, "updated"))
        .expect("update");
    port.delete_row("Table1", 1).expect("delete");
    let after = port.read_rows("Table1", all_rows()).expect("read after");
    assert_eq!(after.len(), 2, "3 rows minus the delete");
    assert_eq!(
        after[0][0],
        FieldValue::Text("updated".into()),
        "the update persists"
    );
    assert_eq!(
        after[1][0],
        FieldValue::Text("writer".into()),
        "the insert persists"
    );
    port.close();

    let reopened = opened(&scratch.0);
    let rows = reopened
        .read_rows("Table1", all_rows())
        .expect("reopen reads");
    assert_eq!(rows, after, "every row operation survived the reopen");
}

#[test]
fn create_alter_and_drop_round_trip_on_an_ace_copy() {
    let scratch = Scratch(scratch_copy("lifecycle"));

    // 1. Create + insert persist.
    let mut port = opened(&scratch.0);
    port.create_table(&people()).expect("create People");
    port.insert_row(
        "People",
        &[
            FieldValue::Integer(7),
            FieldValue::Text("Ada".into()),
            FieldValue::Boolean(true),
        ],
    )
    .expect("insert");
    port.close();

    // 2. The created table survives the reopen (format still ACE).
    let mut port = opened(&scratch.0);
    assert!(
        port.tables().iter().any(|table| table.name == "People"),
        "{:?}",
        port.tables()
    );

    // 3. ALTER TABLE persists.
    port.alter_table(
        "People",
        SchemaChange::AddField {
            field: FieldDef::new("extra", FieldKind::Text { max_len: 20 }),
        },
    )
    .expect("add a field");
    port.close();

    let mut port = opened(&scratch.0);
    let people = port
        .tables()
        .into_iter()
        .find(|table| table.name == "People")
        .expect("People survives");
    assert_eq!(people.fields.len(), 4, "the altered schema persists");
    let rows = port
        .read_rows("People", all_rows())
        .expect("rows survive the alter");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        rows[0][3],
        FieldValue::Null,
        "the appended field reads Null"
    );

    // 4. The drop persists; the fixture catalog stays intact.
    port.drop_table("People").expect("drop People");
    port.close();
    let port = opened(&scratch.0);
    assert!(
        !port.tables().iter().any(|table| table.name == "People"),
        "People is gone"
    );
    assert!(
        port.tables().iter().any(|table| table.name == "Table1"),
        "the fixture catalog survives"
    );
}

#[test]
fn ace_write_unsupported_cases_stay_precise() {
    let scratch = Scratch(scratch_copy("unsupported"));
    let mut port = opened(&scratch.0);

    let attachment = TableDef::new(
        "Attach",
        vec![FieldDef::new(
            "data",
            FieldKind::Binary(BinaryKind::Attachment),
        )],
    );
    let error = port
        .create_table(&attachment)
        .expect_err("attachments are a complex-value field");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    let replication = TableDef::new(
        "Auto",
        vec![FieldDef::new(
            "id",
            FieldKind::AutoNumber(AutoKind::ReplicationId),
        )],
    );
    let error = port
        .create_table(&replication)
        .expect_err("GUID AutoNumbers stay unsupported");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");

    let error = port
        .insert_row("Missing", &[FieldValue::Null])
        .expect_err("unknown table");
    assert!(matches!(error, PortError::TableNotFound(_)), "{error:?}");
}
