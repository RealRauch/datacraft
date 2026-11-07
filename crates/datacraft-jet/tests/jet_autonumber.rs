//! AutoNumber maintenance contract (step 24.1, item M8).
//!
//! The writer assigns engine values on insert (Null = engine-assigned),
//! persists the table's next counter in the TDEF head and keeps it across a
//! reopen; user-supplied AutoNumber values stay rejected and the GUID
//! flavor stays a precise `Unsupported`.

use std::path::{Path, PathBuf};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_jet::{
    StorageValue,
    jet::{Jet4TableReader, Jet4Writer, JetColumn, JetTableDef, JetType},
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("testV2000.mdb")
}

/// Private copy of the pinned fixture — writes never touch the original.
fn fixture_copy(stem: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "datacraft-autonumber-{}-{stem}.mdb",
        std::process::id()
    ));
    std::fs::copy(fixture(), &path).expect("copy the fixture");
    path
}

fn autonumber_table() -> TableDef {
    TableDef::new(
        "AutoId",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
            FieldDef::new("note", FieldKind::Text { max_len: 20 }),
        ],
    )
}

#[test]
fn the_writer_assigns_and_persists_the_counter() {
    let path = fixture_copy("writer");
    let file = std::fs::read(&path).expect("copy readable");
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "AutoWriter",
            &[
                JetColumn::new("id", JetType::LongInt, 0x05, 0, 0, 4),
                JetColumn::new("note", JetType::Text, 0x02, 1, 0, 40),
            ],
        )
        .expect("table created");
    for note in ["first", "second"] {
        writer
            .insert_row(
                tdef,
                &[
                    StorageValue::Null, // engine-assigned
                    StorageValue::Text(note.to_owned()),
                ],
            )
            .expect("row inserted");
    }
    let written = writer.into_file();
    let reader = Jet4TableReader::new(written.clone()).expect("written file opens");
    let data = reader.open_table("AutoWriter").expect("table reads");
    assert_eq!(data.rows[0][0], StorageValue::Int(1));
    assert_eq!(data.rows[1][0], StorageValue::Int(2));
    let def = JetTableDef::decode(&written, tdef).expect("TDEF decodes");
    assert_eq!(
        def.autonumber, 2,
        "the last assigned value persists in the TDEF (the engine's seed \
         semantics, observed 2026-11-07 via the DAO oracle)"
    );
    let _ = std::fs::remove_file(&path);
}

/// The engine stores the TDEF seed as "last assigned" and takes the next
/// value from seed + 1 (observed 2026-11-07 via the DAO oracle: after the
/// engine inserted ids 1..3, DataCraft's insert must take 4 — re-using the
/// seed collided with the engine's row). This test simulates that
/// engine-written state by patching the seed dword in a copy.
#[test]
fn an_engine_advanced_seed_assigns_one_past_it() {
    let path = fixture_copy("advanced");
    let file = std::fs::read(&path).expect("copy readable");
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "AutoAdvanced",
            &[
                JetColumn::new("id", JetType::LongInt, 0x05, 0, 0, 4),
                JetColumn::new("note", JetType::Text, 0x02, 1, 0, 40),
            ],
        )
        .expect("table created");
    writer
        .insert_row(
            tdef,
            &[
                StorageValue::Null,
                StorageValue::Text("seeded".to_owned()),
            ],
        )
        .expect("row inserted");
    std::fs::write(&path, writer.into_file()).expect("write the copy");

    // Simulate the engine having assigned ids up to 3: the seed dword sits
    // at page + 8 (block head) + 12 (the decode offset of `autonumber`).
    let seed_at = tdef as usize * 4096 + 8 + 12;
    let mut patched = std::fs::read(&path).expect("written file readable");
    patched[seed_at..seed_at + 4].copy_from_slice(&3u32.to_le_bytes());
    std::fs::write(&path, &patched).expect("patch the seed");

    let mut writer = Jet4Writer::new(patched).expect("writer reopens");
    writer
        .insert_row(
            tdef,
            &[
                StorageValue::Null,
                StorageValue::Text("continues".to_owned()),
            ],
        )
        .expect("row inserted");
    let written = writer.into_file();
    let reader = Jet4TableReader::new(written.clone()).expect("written file opens");
    let data = reader.open_table("AutoAdvanced").expect("table reads");
    assert_eq!(
        data.rows[1][0],
        StorageValue::Int(4),
        "the insert continues one past the engine's last assigned value"
    );
    let def = JetTableDef::decode(&written, tdef).expect("TDEF decodes");
    assert_eq!(
        def.autonumber, 4,
        "the assigned value is stored as the new seed"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_port_assigns_values_that_survive_a_reopen() {
    let path = fixture_copy("port");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    port.create_table(&autonumber_table())
        .expect("create with AutoNumber");
    for note in ["one", "two"] {
        port.insert_row(
            "AutoId",
            &[FieldValue::Null, FieldValue::Text(note.to_owned())],
        )
        .expect("insert with engine-assigned id");
    }
    let rows = port
        .read_rows(
            "AutoId",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("rows read");
    assert_eq!(rows[0][0], FieldValue::Integer(1));
    assert_eq!(rows[1][0], FieldValue::Integer(2));
    port.close();

    // Reopen: the counter continued from the persisted TDEF value.
    port.open(&path, None).expect("reopen");
    port.insert_row(
        "AutoId",
        &[FieldValue::Null, FieldValue::Text("three".to_owned())],
    )
    .expect("third insert");
    let rows = port
        .read_rows(
            "AutoId",
            RowRange {
                start: 0,
                len: usize::MAX,
            },
        )
        .expect("rows read after reopen");
    assert_eq!(rows[2][0], FieldValue::Integer(3));
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn user_supplied_autonumber_values_stay_rejected() {
    let path = fixture_copy("user");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    port.create_table(&autonumber_table()).expect("create");
    let error = port
        .insert_row(
            "AutoId",
            &[
                FieldValue::Integer(42),
                FieldValue::Text("manual".to_owned()),
            ],
        )
        .expect_err("user-supplied ids are rejected");
    assert!(error.to_string().contains("AutoNumber"), "{error}");
    port.close();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn guid_autonumbers_stay_a_precise_unsupported() {
    let path = fixture_copy("guid");
    let mut port = Jet4Port::new();
    port.open(&path, None).expect("copy opens");
    let def = TableDef::new(
        "Guids",
        vec![
            FieldDef::new("id", FieldKind::AutoNumber(AutoKind::ReplicationId)),
            FieldDef::new("value", FieldKind::Number(NumericKind::LongInteger)),
        ],
    );
    let error = port
        .create_table(&def)
        .expect_err("ReplicationId stays unsupported");
    assert!(matches!(error, PortError::Unsupported { .. }), "{error:?}");
    port.close();
    let _ = std::fs::remove_file(&path);
}

/// Oracle gate (CI linux): mdb-export must see the assigned values.
#[test]
fn mdb_export_sees_the_assigned_values() {
    if std::env::var_os("DATACRAFT_ORACLE_MDB").is_none() {
        eprintln!("skipped: DATACRAFT_ORACLE_MDB not set");
        return;
    }
    if std::process::Command::new("mdb-export")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipped: mdb-export not installed");
        return;
    }
    let path = fixture_copy("export");
    let file = std::fs::read(&path).expect("copy readable");
    let mut writer = Jet4Writer::new(file).expect("writer opens");
    let tdef = writer
        .create_table(
            "AutoExport",
            &[
                JetColumn::new("id", JetType::LongInt, 0x05, 0, 0, 4),
                JetColumn::new("note", JetType::Text, 0x02, 1, 0, 40),
            ],
        )
        .expect("table created");
    for note in ["first", "second"] {
        writer
            .insert_row(
                tdef,
                &[StorageValue::Null, StorageValue::Text(note.to_owned())],
            )
            .expect("row inserted");
    }
    std::fs::write(&path, writer.into_file()).expect("write the copy");
    let output = std::process::Command::new("mdb-export")
        .arg(&path)
        .arg("AutoExport")
        .output()
        .expect("mdb-export runs");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1,first"), "{stdout}");
    assert!(stdout.contains("2,second"), "{stdout}");
    let _ = std::fs::remove_file(&path);
}
