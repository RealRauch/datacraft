//! Jet/ACE encryption and database passwords (step 16.2, item M5).
//!
//! Facts per `docs/FORMAT-JET.md` §"Encryption and passwords": the page-0
//! header region (0x18, 126/128 bytes) is RC4-encrypted with the fixed key
//! `C7 DA 39 6B`; a non-zero database key at 0x3e encrypts every page after
//! page 0 with `RC4(db_key XOR page_number)`. Jet3/Jet4 store the database
//! password obfuscated in the header (Jet3: fixed XOR, Jet4: creation-date
//! XOR mask); ACE 2007+ uses Office encryption and answers a precise
//! `Unsupported`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_jet::StorageError;
use datacraft_jet::jet::crypto::{self, HEADER_RC4_KEY, PageEncryption};
use datacraft_jet::jet::{FileHeader, Jet4TableReader};
use datacraft_jet::port_adapter::Jet4Port;
use datacraft_storage::detecting::DetectingPort;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

fn open(name: &str, password: Option<&str>) -> Result<DetectingPort, PortError> {
    let mut port = DetectingPort::new();
    port.open(&fixture(name), password)?;
    Ok(port)
}

fn all_rows(port: &impl StoragePort, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .unwrap_or_else(|e| panic!("read {table}: {e}"))
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn scratch_path(stem: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-jet-encryption-{}-{n}-{stem}.mdb",
        std::process::id()
    ))
}

/// Builds a password-protected, page-encrypted copy of the Jet4 fixture.
fn encrypted_jet4_copy(stem: &str, db_key: u32, password: &str) -> Scratch {
    let mut bytes = std::fs::read(fixture("testV2000.mdb")).expect("fixture");
    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("header");
    // Page 0 header region: decrypt, set the database key and the masked
    // password, then let `encrypt_file` re-encrypt it with the fixed key.
    let mut region = bytes[0x18..0x18 + 128].to_vec();
    crypto::rc4(&HEADER_RC4_KEY, &mut region);
    region[0x3e - 0x18..0x42 - 0x18].copy_from_slice(&db_key.to_le_bytes());
    let date = f64::from_le_bytes(
        region[0x72 - 0x18..0x7a - 0x18]
            .try_into()
            .expect("creation date"),
    );
    let mask = (date as i64 as u32).to_le_bytes();
    let mut field = [0u8; 40];
    for (index, unit) in password.encode_utf16().enumerate() {
        field[index * 2..index * 2 + 2].copy_from_slice(&unit.to_le_bytes());
    }
    for (index, byte) in field.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    region[0x42 - 0x18..0x6a - 0x18].copy_from_slice(&field);
    bytes[0x18..0x18 + 128].copy_from_slice(&region);
    crypto::encrypt_file(&mut bytes, &header, Some(&PageEncryption { db_key }))
        .expect("encrypt the copy");
    let path = scratch_path(stem);
    std::fs::write(&path, &bytes).expect("write the encrypted copy");
    Scratch(path)
}

#[test]
fn a_password_protected_jet4_file_requires_the_password() {
    let err = open("passwordV2000.mdb", None).expect_err("password is required");
    assert!(matches!(err, PortError::PasswordRequired { .. }), "{err:?}");
}

#[test]
fn the_correct_password_opens_and_matches_the_plain_control() {
    let with = open("passwordV2000.mdb", Some("password")).expect("correct password");
    let without = open("passwordV2000-none.mdb", None).expect("plain control");
    assert_eq!(with.tables(), without.tables());
    for table in with.tables() {
        assert_eq!(
            all_rows(&with, &table.name),
            all_rows(&without, &table.name),
            "table {}",
            table.name
        );
    }
}

#[test]
fn a_wrong_password_is_rejected_precisely() {
    let err = open("passwordV2000.mdb", Some("wrong")).expect_err("wrong password");
    assert!(matches!(err, PortError::WrongPassword { .. }), "{err:?}");
}

#[test]
fn an_unprotected_file_ignores_a_supplied_password() {
    let port = open("passwordV2000-none.mdb", Some("anything")).expect("plain file opens");
    assert!(!port.tables().is_empty());
}

#[test]
fn ace_office_encryption_is_a_precise_unsupported() {
    for name in ["passwordV2010.accdb", "officeAgile42.accdb"] {
        let err = open(name, Some("password")).expect_err("ACE encryption is unsupported");
        match err {
            PortError::Unsupported { reason } => {
                assert!(reason.contains("Office encryption"), "{name}: {reason}");
            }
            other => panic!("{name}: expected Unsupported, got {other:?}"),
        }
    }
}

#[test]
fn the_header_region_decodes_to_code_page_and_creation_date() {
    let bytes = std::fs::read(fixture("passwordV2000.mdb")).expect("fixture");
    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("header");
    let info = crypto::decode_header_region(&bytes, &header).expect("header region");
    assert_eq!(info.code_page, 1252);
    assert_eq!(info.db_key, 0);
    assert!(info.password_protected);
    assert!(!info.office_encrypted);
    assert!(info.creation_date > 40_000.0);
}

#[test]
fn page_encryption_round_trips_through_reads_and_writes() {
    let scratch = encrypted_jet4_copy("page-key", 0x1234_5678, "secret");
    let raw = std::fs::read(&scratch.0).expect("encrypted file");
    // The TDEF page 2 is no longer recognizable as plaintext.
    assert_ne!(raw[2 * 4096], 0x02, "page 2 must be encrypted on disk");

    let mut port = Jet4Port::new();
    port.open(&scratch.0, Some("secret"))
        .expect("the password opens the encrypted copy");
    let before = all_rows(&port, "Table1").len();
    assert!(before > 0, "the fixture rows decode");
    port.insert_row(
        "Table1",
        &[
            FieldValue::Text("encrypted".into()),
            FieldValue::Text("row".into()),
            FieldValue::Integer(1),
            FieldValue::Integer(2),
            FieldValue::Integer(3),
            FieldValue::Float(4.5),
            FieldValue::DateTime(45_000.0),
            FieldValue::Currency(12_500),
            FieldValue::Boolean(true),
        ],
    )
    .expect("writes stay available on an encrypted file");
    port.close();

    let raw = std::fs::read(&scratch.0).expect("rewritten file");
    assert_ne!(raw[2 * 4096], 0x02, "the writer re-encrypts the file");

    let mut reopened = Jet4Port::new();
    reopened
        .open(&scratch.0, Some("secret"))
        .expect("the rewritten file still opens");
    let rows = all_rows(&reopened, "Table1");
    assert_eq!(rows.len(), before + 1);
    assert_eq!(rows[before][0], FieldValue::Text("encrypted".into()));
}

#[test]
fn a_missing_or_wrong_password_on_an_encrypted_copy_is_precise() {
    let scratch = encrypted_jet4_copy("page-key-errors", 0x0BAD_F00D, "secret");
    let mut port = Jet4Port::new();
    assert!(matches!(
        port.open(&scratch.0, None),
        Err(PortError::PasswordRequired { .. })
    ));
    assert!(matches!(
        port.open(&scratch.0, Some("wrong")),
        Err(PortError::WrongPassword { .. })
    ));
}

#[test]
fn jet3_password_and_page_encryption_decode_through_the_reader() {
    // A minimal Jet3 file: one TDEF page (one text column) and one data page.
    const PAGE: usize = 2048;
    let mut file = vec![0u8; PAGE * 3];
    file[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    file[4..20].copy_from_slice(b"Standard Jet DB\0");
    file[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
    // TDEF page 1: 'VC' header, 35-byte block, one 18-byte descriptor.
    let base = PAGE;
    file[base] = 0x02;
    file[base + 1] = 0x01;
    file[base + 2..base + 4].copy_from_slice(b"VC");
    let mut block: Vec<u8> = Vec::new();
    block.extend_from_slice(&0u32.to_le_bytes()); // tdef_len (patched below)
    block.extend_from_slice(&0u32.to_le_bytes()); // num_rows
    block.extend_from_slice(&0u32.to_le_bytes()); // autonumber
    block.push(0x4E);
    block.extend_from_slice(&1u16.to_le_bytes()); // max_cols
    block.extend_from_slice(&1u16.to_le_bytes()); // num_var_cols
    block.extend_from_slice(&1u16.to_le_bytes()); // num_cols
    block.extend_from_slice(&0u32.to_le_bytes()); // num_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // num_real_idx
    block.extend_from_slice(&0u32.to_le_bytes()); // used_pages
    block.extend_from_slice(&0u32.to_le_bytes()); // free_pages
    block.push(0x0A); // Text
    block.extend_from_slice(&0u16.to_le_bytes()); // col_num
    block.extend_from_slice(&0u16.to_le_bytes()); // offset_V
    block.extend_from_slice(&0u16.to_le_bytes()); // col_id
    block.extend_from_slice(&0x0409u16.to_le_bytes()); // sort order
    block.extend_from_slice(&0x04E4u16.to_le_bytes()); // code page
    block.extend_from_slice(&0u16.to_le_bytes()); // unknown
    block.push(0x02); // variable, nullable
    block.extend_from_slice(&0u16.to_le_bytes()); // offset_F
    block.extend_from_slice(&100u16.to_le_bytes()); // len
    block.push(5);
    block.extend_from_slice(b"Name\0");
    block.extend_from_slice(&0xFFFFu16.to_le_bytes());
    let len = block.len() as u32;
    block[..4].copy_from_slice(&len.to_le_bytes());
    file[base + 8..base + 8 + block.len()].copy_from_slice(&block);
    // Data page 2: one row [num_cols=1][var "alpha"][eod][var_table][var_len][mask].
    let data = 2 * PAGE;
    file[data] = 0x01;
    file[data + 1] = 0x01;
    file[data + 4..data + 8].copy_from_slice(&1u32.to_le_bytes());
    file[data + 8..data + 10].copy_from_slice(&1u16.to_le_bytes());
    let mut row = vec![1u8];
    let start = row.len() as u8;
    row.extend_from_slice(b"alpha");
    row.push(row.len() as u8); // eod
    row.push(start); // var_table
    row.push(1); // var_len
    row.push(0b1); // null mask
    let row_start = PAGE - row.len();
    file[data + row_start..data + PAGE].copy_from_slice(&row);
    file[data + 10..data + 12].copy_from_slice(&(row_start as u16).to_le_bytes());
    file[data + 2..data + 4].copy_from_slice(&((row_start - 12) as u16).to_le_bytes());

    let header = FileHeader::decode(&file, file.len() as u64).expect("Jet3 header");
    // Jet3 password field: plain text inside the encrypted header region.
    let mut region = file[0x18..0x18 + 126].to_vec();
    crypto::rc4(&HEADER_RC4_KEY, &mut region);
    let db_key = 0x00C0_FFEEu32;
    region[0x3e - 0x18..0x42 - 0x18].copy_from_slice(&db_key.to_le_bytes());
    let mut field = [0u8; 20];
    field[..6].copy_from_slice(b"secret");
    region[0x42 - 0x18..0x56 - 0x18].copy_from_slice(&field);
    file[0x18..0x18 + 126].copy_from_slice(&region);
    crypto::encrypt_file(&mut file, &header, Some(&PageEncryption { db_key }))
        .expect("encrypt Jet3");

    let err = Jet4TableReader::new_with_password(file.clone(), None).expect_err("no password");
    assert!(matches!(err, StorageError::PasswordRequired), "{err:?}");
    let err = Jet4TableReader::new_with_password(file.clone(), Some("wrong"))
        .expect_err("wrong password");
    assert!(matches!(err, StorageError::WrongPassword), "{err:?}");
    let reader =
        Jet4TableReader::new_with_password(file, Some("secret")).expect("correct password");
    let table = reader.read_table(1).expect("the encrypted page decodes");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(
        table.rows[0][0],
        datacraft_jet::StorageValue::Text("alpha".into())
    );
}
