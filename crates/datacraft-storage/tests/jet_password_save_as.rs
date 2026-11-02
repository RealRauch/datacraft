//! Encrypted Save-As and password changes on real Jet4/ACE files
//! (step 27.2, item M14).
//!
//! Every scenario runs on a byte copy: `save_as` must never touch the
//! source, must produce a file that reopens with exactly the requested
//! password state, and `change_password` must rewrite the open file so
//! the old password stops working. ACE keeps its Office-encryption
//! boundary (`Unsupported`); Jet3 stays read-only.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use datacraft_core::port::{PortError, RowRange, StoragePort};
use datacraft_core::schema::FieldValue;
use datacraft_storage::jet::FileHeader;
use datacraft_storage::jet::crypto::{self, HEADER_RC4_KEY, PageEncryption};
use datacraft_storage::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn scratch_path(stem: &str, extension: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "datacraft-save-as-{}-{n}-{stem}.{extension}",
        std::process::id()
    ))
}

fn copy_of(name: &str, stem: &str) -> Scratch {
    let target = scratch_path(stem, "mdb");
    std::fs::copy(fixture(name), &target).expect("copy the fixture");
    Scratch(target)
}

fn all_rows(port: &impl StoragePort, table: &str) -> Vec<Vec<FieldValue>> {
    port.read_rows(
        table,
        RowRange {
            start: 0,
            len: usize::MAX,
        },
    )
    .expect("read rows")
}

/// Builds a page-encrypted Jet4 copy with `db_key` and `password`.
fn encrypted_copy(stem: &str, db_key: u32, password: &str) -> Scratch {
    let mut bytes = std::fs::read(fixture("testV2000.mdb")).expect("fixture");
    let header = FileHeader::decode(&bytes, bytes.len() as u64).expect("header");
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
    let path = scratch_path(stem, "mdb");
    std::fs::write(&path, &bytes).expect("write the encrypted copy");
    Scratch(path)
}

fn assert_same_tables(left: &impl StoragePort, right: &impl StoragePort) {
    assert_eq!(left.tables(), right.tables());
    for table in left.tables() {
        assert_eq!(
            all_rows(left, &table.name),
            all_rows(right, &table.name),
            "table {}",
            table.name
        );
    }
}

#[test]
fn save_as_removes_the_password_on_a_copy_and_never_touches_the_source() {
    let source = copy_of("passwordV2000.mdb", "source-remove");
    let before = std::fs::read(&source.0).expect("source bytes");
    let target = scratch_path("target-plain", "mdb");

    let mut port = Jet4Port::new();
    port.open(&source.0, Some("password"))
        .expect("the source opens with its password");
    port.save_as(&target, None)
        .expect("the unencrypted copy is written");
    assert_eq!(
        std::fs::read(&source.0).expect("source bytes"),
        before,
        "save-as never rewrites the source"
    );

    let mut copy = Jet4Port::new();
    copy.open(&target, None)
        .expect("the copy opens without a password");
    assert_same_tables(&port, &copy);
    let _ = std::fs::remove_file(&target);
}

#[test]
fn save_as_adds_a_password_to_an_unencrypted_copy() {
    let source = copy_of("passwordV2000-none.mdb", "source-add");
    let target = scratch_path("target-protected", "mdb");

    let mut port = Jet4Port::new();
    port.open(&source.0, None).expect("the plain source opens");
    port.save_as(&target, Some("newpass"))
        .expect("the protected copy is written");

    let mut none = Jet4Port::new();
    assert!(matches!(
        none.open(&target, None),
        Err(PortError::PasswordRequired { .. })
    ));
    let mut wrong = Jet4Port::new();
    assert!(matches!(
        wrong.open(&target, Some("password")),
        Err(PortError::WrongPassword { .. })
    ));
    let mut right = Jet4Port::new();
    right
        .open(&target, Some("newpass"))
        .expect("the new password opens the copy");
    assert_same_tables(&port, &right);
    let _ = std::fs::remove_file(&target);
}

#[test]
fn save_as_reencrypts_a_page_encrypted_source() {
    let source = encrypted_copy("source-keyed", 0x1234_5678, "secret");
    let target = scratch_path("target-unkeyed", "mdb");
    let mut port = Jet4Port::new();
    port.open(&source.0, Some("secret"))
        .expect("the page-encrypted source opens");
    port.save_as(&target, None)
        .expect("the copy is written unencrypted");
    let mut copy = Jet4Port::new();
    copy.open(&target, None)
        .expect("the unencrypted copy opens");
    assert_same_tables(&port, &copy);
    let _ = std::fs::remove_file(&target);
}

#[test]
fn changing_the_password_rewrites_the_open_file() {
    let source = copy_of("passwordV2000.mdb", "change");
    let mut port = Jet4Port::new();
    port.open(&source.0, Some("password"))
        .expect("the source opens");
    port.change_password(Some("changed"))
        .expect("the password changes in place");
    port.close();

    let mut old = Jet4Port::new();
    assert!(matches!(
        old.open(&source.0, Some("password")),
        Err(PortError::WrongPassword { .. })
    ));
    let mut changed = Jet4Port::new();
    changed
        .open(&source.0, Some("changed"))
        .expect("the new password opens the rewritten file");
    assert!(!changed.tables().is_empty());
    changed.close();

    // Removing the protection is the same command with `None`.
    let mut port = Jet4Port::new();
    port.open(&source.0, Some("changed")).expect("reopen");
    port.change_password(None).expect("the password is removed");
    port.close();
    let mut plain = Jet4Port::new();
    plain
        .open(&source.0, None)
        .expect("the unprotected file opens without a password");
    assert!(!plain.tables().is_empty());
}

#[test]
fn save_as_refuses_an_existing_target() {
    let source = copy_of("passwordV2000-none.mdb", "existing-source");
    let target = scratch_path("existing-target", "mdb");
    std::fs::write(&target, b"occupied").expect("seed the target");
    let mut port = Jet4Port::new();
    port.open(&source.0, None).expect("the source opens");
    let error = port
        .save_as(&target, None)
        .expect_err("an existing target is never overwritten");
    assert!(error.to_string().contains("exists"), "{error}");
    assert_eq!(
        std::fs::read(&target).expect("target bytes"),
        b"occupied",
        "the existing target keeps its bytes"
    );
    let _ = std::fs::remove_file(&target);
}

#[test]
fn ace_copies_keep_the_office_encryption_boundary() {
    let source = copy_of("testV2010.accdb", "ace-source");
    let target = scratch_path("ace-target", "accdb");
    let mut port = Jet4Port::new();
    port.open(&source.0, None).expect("the ACE source opens");

    let error = port
        .save_as(&target, Some("secret"))
        .expect_err("ACE Office encryption is not written");
    match error {
        PortError::Unsupported { reason } => {
            assert!(reason.contains("Office encryption"), "{reason}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }

    port.save_as(&target, None)
        .expect("an unencrypted ACE copy is written");
    let mut copy = Jet4Port::new();
    copy.open(&target, None)
        .expect("the unencrypted ACE copy opens");
    assert_same_tables(&port, &copy);

    let error = port
        .change_password(Some("secret"))
        .expect_err("ACE password changes stay unsupported");
    assert!(error.to_string().contains("Office encryption"), "{error}");
    let _ = std::fs::remove_file(&target);
}

#[test]
fn jet3_sources_are_read_only_for_save_as() {
    let source = copy_of("testV1997.mdb", "jet3-source");
    let target = scratch_path("jet3-target", "mdb");
    let mut port = Jet4Port::new();
    port.open(&source.0, None).expect("the Jet3 source opens");
    let error = port.save_as(&target, None).expect_err("Jet3 has no writer");
    assert!(error.to_string().contains("Jet3"), "{error}");
    assert!(!target.exists(), "no partial copy is left behind");
}
