//! Jet/ACE header and page encryption (step 16.2, item M5).
//!
//! Facts per `docs/FORMAT-JET.md` §"Encryption and passwords":
//!
//! - The page-0 region from 0x18 (126 bytes Jet3, 128 bytes Jet4/ACE) is
//!   RC4-"encrypted" with the fixed key `C7 DA 39 6B` — even in unprotected
//!   files, so every read decrypts it and every write re-encrypts it.
//! - A non-zero database key at 0x3e means every page after page 0 is
//!   RC4-encrypted with `db_key XOR page_number` (4 LE bytes).
//! - Jet3 stores the database password at 0x42 (20 bytes) as **plain text**
//!   inside the encrypted header region (the documented fixed XOR key is the
//!   differential-recovery technique, not an in-file mask); Jet4 stores 40
//!   bytes (UTF-16LE, 20 characters) XOR a 4-byte mask derived from the
//!   creation-date double at 0x72.
//! - ACE 2007+ replaces the page cipher with Office encryption (RC4
//!   CryptoAPI or AES); the structure length at 0x299 and/or a non-zero
//!   database key marks it. DataCraft answers a precise `Unsupported`.

use crate::error::StorageError;
use crate::jet::header::FileHeader;
use crate::types::FormatVersion;

/// Fixed RC4 key of the page-0 header region (`0x6b39dac7` little-endian).
pub const HEADER_RC4_KEY: [u8; 4] = [0xC7, 0xDA, 0x39, 0x6B];
/// Page-0 header region length of Jet3.
pub const JET3_HEADER_REGION: usize = 126;
/// Page-0 header region length of Jet4/ACE.
pub const JET4_HEADER_REGION: usize = 128;

/// Page-0 header region start.
const HEADER_START: usize = 0x18;
/// Database key offset (page-0 absolute).
const DB_KEY_OFFSET: usize = 0x3e;
/// Code page offset (page-0 absolute).
const CODE_PAGE_OFFSET: usize = 0x3c;
/// Password field offset (page-0 absolute).
const PASSWORD_OFFSET: usize = 0x42;
/// Creation-date offset (page-0 absolute; Jet4/ACE).
const CREATION_DATE_OFFSET: usize = 0x72;
/// Office-encryption descriptor length offset (ACE, page 0).
const OFFICE_DESCRIPTOR_OFFSET: usize = 0x299;
/// Jet3 password field length.
const JET3_PASSWORD_LEN: usize = 20;
/// Jet4/ACE password field length.
const JET4_PASSWORD_LEN: usize = 40;

/// Page encryption state of an open database (a non-zero database key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageEncryption {
    /// Database key stored at 0x3e.
    pub db_key: u32,
}

/// Decoded page-0 header facts (the password itself is never exposed).
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderInfo {
    /// Default database code page (0x3c).
    pub code_page: u16,
    /// Database key (0x3e); non-zero means pages are encrypted.
    pub db_key: u32,
    /// Whether the password field is non-empty.
    pub password_protected: bool,
    /// ACE Office encryption (RC4 CryptoAPI/AES) — not supported.
    pub office_encrypted: bool,
    /// Creation date (OLE automation days, Jet4/ACE).
    pub creation_date: f64,
}

/// Decoded header plus the stored password (internal only).
struct DecodedHeader {
    info: HeaderInfo,
    password: String,
}

/// RC4 (KSA + PRGA) applied in place.
pub fn rc4(key: &[u8], data: &mut [u8]) {
    let mut state: [u8; 256] = [0; 256];
    for (index, slot) in state.iter_mut().enumerate() {
        *slot = index as u8;
    }
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + usize::from(state[i]) + usize::from(key[i % key.len()])) & 0xFF;
        state.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    for byte in data.iter_mut() {
        i = (i + 1) & 0xFF;
        j = (j + usize::from(state[i])) & 0xFF;
        state.swap(i, j);
        let k = state[(usize::from(state[i]) + usize::from(state[j])) & 0xFF];
        *byte ^= k;
    }
}

/// Decrypts a copy of the page-0 header region and returns its facts.
pub fn decode_header_region(file: &[u8], header: &FileHeader) -> Result<HeaderInfo, StorageError> {
    let len = header_region_len(header.format);
    let mut region = file
        .get(HEADER_START..HEADER_START + len)
        .ok_or_else(|| corrupt("file shorter than the page-0 header region"))?
        .to_vec();
    rc4(&HEADER_RC4_KEY, &mut region);
    Ok(decode_region(&region, header.format, office_descriptor_len(file))?.info)
}

/// Decrypts the header region in place, validates the password and decrypts
/// every page when the database key is non-zero. Returns the page-encryption
/// state for the writer's re-encryption pass.
pub fn decrypt_file(
    file: &mut [u8],
    header: &FileHeader,
    password: Option<&str>,
) -> Result<Option<PageEncryption>, StorageError> {
    let len = header_region_len(header.format);
    let office_len = office_descriptor_len(file);
    let region = file
        .get_mut(HEADER_START..HEADER_START + len)
        .ok_or_else(|| corrupt("file shorter than the page-0 header region"))?;
    rc4(&HEADER_RC4_KEY, region);
    let decoded = decode_region(region, header.format, office_len)?;
    if decoded.info.office_encrypted {
        return Err(StorageError::Unsupported {
            reason: "ACE Office encryption (RC4 CryptoAPI/AES) is not supported — \
                     DataCraft reads Jet3/Jet4 RC4 databases only"
                .to_owned(),
        });
    }
    if !decoded.password.is_empty() {
        match password {
            None => return Err(StorageError::PasswordRequired),
            Some(given) if given != decoded.password => return Err(StorageError::WrongPassword),
            Some(_) => {}
        }
    }
    let encryption = (decoded.info.db_key != 0).then_some(PageEncryption {
        db_key: decoded.info.db_key,
    });
    if let Some(encryption) = encryption {
        crypt_pages(file, header, encryption.db_key);
    }
    Ok(encryption)
}

/// Encrypts a decrypted file for persistence: the header region always (it
/// is RC4-"encrypted" even in unprotected files), every page after page 0
/// when a database key is present.
pub fn encrypt_file(
    file: &mut [u8],
    header: &FileHeader,
    page_encryption: Option<&PageEncryption>,
) -> Result<(), StorageError> {
    let len = header_region_len(header.format);
    let region = file
        .get_mut(HEADER_START..HEADER_START + len)
        .ok_or_else(|| corrupt("file shorter than the page-0 header region"))?;
    rc4(&HEADER_RC4_KEY, region);
    if let Some(encryption) = page_encryption {
        crypt_pages(file, header, encryption.db_key);
    }
    Ok(())
}

/// Encrypts/decrypts every page after page 0 (RC4 is symmetric).
fn crypt_pages(file: &mut [u8], header: &FileHeader, db_key: u32) {
    for page in 1..header.page_count as u32 {
        let start = page as usize * header.page_size;
        let end = start + header.page_size;
        if let Some(slice) = file.get_mut(start..end) {
            rc4(&(db_key ^ page).to_le_bytes(), slice);
        }
    }
}

/// Parses the decrypted header region.
fn decode_region(
    region: &[u8],
    format: FormatVersion,
    office_len: u16,
) -> Result<DecodedHeader, StorageError> {
    let code_page = u16::from_le_bytes(read::<2>(region, CODE_PAGE_OFFSET - HEADER_START)?);
    let db_key = u32::from_le_bytes(read::<4>(region, DB_KEY_OFFSET - HEADER_START)?);
    let creation_date = f64::from_le_bytes(read::<8>(region, CREATION_DATE_OFFSET - HEADER_START)?);
    let password = match format {
        FormatVersion::Jet3 => jet3_password(region)?,
        FormatVersion::Jet4 | FormatVersion::Ace => jet4_password(region, creation_date)?,
    };
    let password_protected = !password.is_empty();
    let office_encrypted =
        format == FormatVersion::Ace && (db_key != 0 || office_len != 0 || password_protected);
    Ok(DecodedHeader {
        info: HeaderInfo {
            code_page,
            db_key,
            password_protected,
            office_encrypted,
            creation_date,
        },
        password,
    })
}

/// Jet3 password field: plain CP1252 text inside the encrypted header
/// region, NUL/space padded.
fn jet3_password(region: &[u8]) -> Result<String, StorageError> {
    let start = PASSWORD_OFFSET - HEADER_START;
    let bytes = &region[start..start + JET3_PASSWORD_LEN];
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(JET3_PASSWORD_LEN);
    Ok(crate::jet::jet3::cp1252(&bytes[..end])
        .trim_end_matches(' ')
        .to_owned())
}

/// Jet4/ACE password field: creation-date XOR mask, UTF-16LE text.
fn jet4_password(region: &[u8], creation_date: f64) -> Result<String, StorageError> {
    let mask = (creation_date as i64 as u32).to_le_bytes();
    let start = PASSWORD_OFFSET - HEADER_START;
    let mut units = Vec::with_capacity(JET4_PASSWORD_LEN / 2);
    for index in 0..JET4_PASSWORD_LEN / 2 {
        let low = region[start + 2 * index] ^ mask[(2 * index) % 4];
        let high = region[start + 2 * index + 1] ^ mask[(2 * index + 1) % 4];
        let unit = u16::from_le_bytes([low, high]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    Ok(String::from_utf16_lossy(&units))
}

fn header_region_len(format: FormatVersion) -> usize {
    match format {
        FormatVersion::Jet3 => JET3_HEADER_REGION,
        FormatVersion::Jet4 | FormatVersion::Ace => JET4_HEADER_REGION,
    }
}

/// The ACE Office-encryption descriptor length (0 for Jet3/Jet4).
fn office_descriptor_len(file: &[u8]) -> u16 {
    file.get(OFFICE_DESCRIPTOR_OFFSET..OFFICE_DESCRIPTOR_OFFSET + 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .unwrap_or(0)
}

fn read<const N: usize>(region: &[u8], at: usize) -> Result<[u8; N], StorageError> {
    region
        .get(at..at + N)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| corrupt(&format!("truncated header region at offset {at}")))
}

fn corrupt(detail: &str) -> StorageError {
    StorageError::Corrupt {
        context: "encryption".into(),
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
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
}
