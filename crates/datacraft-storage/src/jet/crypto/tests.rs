use super::*;

#[test]
fn rc4_matches_the_reference_vector() {
    let mut data = b"Plaintext".to_vec();
    rc4(b"Key", &mut data);
    assert_eq!(
        data,
        vec![0xBB, 0xF3, 0x16, 0xE8, 0xD9, 0x40, 0xAF, 0x0A, 0xD3]
    );
    rc4(b"Key", &mut data);
    assert_eq!(data, b"Plaintext");
}

#[test]
fn the_jet4_mask_is_the_creation_date_integer() {
    let date = 44_568.676_921_724_54f64;
    let mask = (date as i64 as u32).to_le_bytes();
    assert_eq!(mask, [0x18, 0xAE, 0x00, 0x00]);
}

#[test]
fn jet3_passwords_are_plain_text_in_the_decrypted_header() {
    let mut region = [0u8; JET3_HEADER_REGION];
    region[PASSWORD_OFFSET - HEADER_START..PASSWORD_OFFSET - HEADER_START + 6]
        .copy_from_slice(b"secret");
    assert_eq!(jet3_password(&region).unwrap(), "secret");
    assert_eq!(jet3_password(&[0u8; JET3_HEADER_REGION]).unwrap(), "");
}

#[test]
fn jet4_passwords_unmask_with_the_date_mask() {
    let date = 44_568.0f64;
    let mask = (date as i64 as u32).to_le_bytes();
    let mut region = [0u8; JET4_HEADER_REGION];
    let start = PASSWORD_OFFSET - HEADER_START;
    // The whole field is plaintext XOR the mask; an empty password is
    // therefore the mask itself.
    for index in 0..JET4_PASSWORD_LEN {
        region[start + index] = mask[index % 4];
    }
    for (index, unit) in "password".encode_utf16().enumerate() {
        let bytes = unit.to_le_bytes();
        region[start + 2 * index] = bytes[0] ^ mask[(2 * index) % 4];
        region[start + 2 * index + 1] = bytes[1] ^ mask[(2 * index + 1) % 4];
    }
    assert_eq!(jet4_password(&region, date).unwrap(), "password");
    let mut empty = [0u8; JET4_HEADER_REGION];
    for index in 0..JET4_PASSWORD_LEN {
        empty[start + index] = mask[index % 4];
    }
    assert_eq!(jet4_password(&empty, date).unwrap(), "");
}

#[test]
fn a_truncated_header_region_is_corrupt() {
    let header = FileHeader {
        format: FormatVersion::Jet4,
        version: 1,
        page_size: 4096,
        page_count: 1,
    };
    let err = decode_header_region(&[0u8; 0x20], &header).expect_err("truncated");
    assert!(matches!(err, StorageError::Corrupt { .. }), "{err:?}");
}

#[test]
fn page_encryption_round_trips_in_memory() {
    let header = FileHeader {
        format: FormatVersion::Jet4,
        version: 1,
        page_size: 4096,
        page_count: 3,
    };
    let mut file = vec![0u8; 3 * 4096];
    file[2 * 4096] = 0x02;
    let encryption = PageEncryption {
        db_key: 0x1234_5678,
    };
    // The decrypted header region carries the database key.
    file[DB_KEY_OFFSET..DB_KEY_OFFSET + 4].copy_from_slice(&encryption.db_key.to_le_bytes());
    let original = file.clone();
    encrypt_file(&mut file, &header, Some(&encryption)).expect("encrypt");
    assert_ne!(file[2 * 4096], 0x02);
    let mut decrypted = file;
    let state = decrypt_file(&mut decrypted, &header, None).expect("decrypt");
    assert_eq!(state, Some(encryption));
    assert_eq!(decrypted, original);
}

fn jet4_file() -> (Vec<u8>, FileHeader) {
    let header = FileHeader {
        format: FormatVersion::Jet4,
        version: 1,
        page_size: 4096,
        page_count: 2,
    };
    let mut file = vec![0u8; 2 * 4096];
    file[0x72..0x7a].copy_from_slice(&44_568.0f64.to_le_bytes());
    (file, header)
}

#[test]
fn a_written_password_round_trips_through_decrypt_file() {
    let (mut file, header) = jet4_file();
    let encryption = set_password(&mut file, &header, Some("secret"))
        .expect("write the password")
        .expect("a non-empty password encrypts pages");
    assert_eq!(encryption.db_key, DEFAULT_DB_KEY);
    encrypt_file(&mut file, &header, Some(&encryption)).expect("encrypt");
    let mut missing = file.clone();
    assert!(matches!(
        decrypt_file(&mut missing, &header, None),
        Err(StorageError::PasswordRequired)
    ));
    let mut wrong = file.clone();
    assert!(matches!(
        decrypt_file(&mut wrong, &header, Some("other")),
        Err(StorageError::WrongPassword)
    ));
    let mut right = file;
    let state = decrypt_file(&mut right, &header, Some("secret")).expect("decrypt");
    assert_eq!(state, Some(encryption));
}

#[test]
fn clearing_the_password_clears_the_page_key() {
    let (mut file, header) = jet4_file();
    let encryption = set_password(&mut file, &header, Some("secret")).expect("set");
    encrypt_file(&mut file, &header, encryption.as_ref()).expect("encrypt");
    let mut decrypted = file;
    decrypt_file(&mut decrypted, &header, Some("secret")).expect("decrypt");
    assert_eq!(
        set_password(&mut decrypted, &header, None).expect("clear"),
        None
    );
    encrypt_file(&mut decrypted, &header, None).expect("encrypt header");
    assert_eq!(
        decrypt_file(&mut decrypted, &header, None).expect("plain file"),
        None
    );
}

#[test]
fn unsupported_password_shapes_are_precise() {
    let (mut jet4, header) = jet4_file();
    let long = "x".repeat(JET4_PASSWORD_LEN / 2 + 1);
    let error = set_password(&mut jet4, &header, Some(&long)).expect_err("too long");
    assert!(error.to_string().contains("limited"), "{error}");
    let error = set_password(&mut jet4, &header, Some("a\0b")).expect_err("NUL");
    assert!(error.to_string().contains("NUL"), "{error}");

    let ace = FileHeader {
        format: FormatVersion::Ace,
        version: 2,
        page_size: 4096,
        page_count: 2,
    };
    let mut file = vec![0u8; 2 * 4096];
    let error = set_password(&mut file, &ace, Some("secret")).expect_err("ACE");
    assert!(error.to_string().contains("Office encryption"), "{error}");
    assert_eq!(
        set_password(&mut file, &ace, None).expect("plain ACE"),
        None
    );

    let jet3 = FileHeader {
        format: FormatVersion::Jet3,
        version: 0,
        page_size: 2048,
        page_count: 2,
    };
    let error = set_password(&mut file, &jet3, None).expect_err("Jet3");
    assert!(error.to_string().contains("Jet3"), "{error}");
}
