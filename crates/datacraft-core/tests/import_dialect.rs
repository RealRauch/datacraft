//! The shared CSV decode and delimiter-sniff path (step 80.2, item H59):
//! BOM handling, UTF-16 and Windows-1252 decoding and the delimiter
//! choice are one deterministic path for the import flow and the CSV
//! driver. The strict UTF-8 branch stays first, so existing UTF-8
//! sources keep their exact results.

use std::path::PathBuf;

use datacraft_core::import::{CsvEncoding, read_csv_file};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("datacraft-dialect-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.join(name)
}

fn utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn utf16be(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

#[test]
fn utf8_sources_report_utf8_and_comma_with_or_without_a_bom() {
    let plain = scratch("plain.csv");
    std::fs::write(&plain, "a,b\n1,x\n").expect("write");
    let (table, dialect) = read_csv_file(&plain).expect("parse");
    assert_eq!(dialect.encoding, CsvEncoding::Utf8);
    assert_eq!(dialect.delimiter, b',');
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.rows, vec![vec!["1".to_owned(), "x".to_owned()]]);

    let bom = scratch("bom.csv");
    let mut bytes = b"\xef\xbb\xbf".to_vec();
    bytes.extend_from_slice(b"a,b\n1,x\n");
    std::fs::write(&bom, bytes).expect("write");
    let (table, dialect) = read_csv_file(&bom).expect("parse");
    assert_eq!(dialect.encoding, CsvEncoding::Utf8);
    assert_eq!(dialect.delimiter, b',');
    assert_eq!(table.fields[0].name, "a", "the BOM is stripped");
}

#[test]
fn a_utf16le_bom_source_decodes_to_the_same_table() {
    let path = scratch("utf16le.csv");
    std::fs::write(&path, utf16le("\u{feff}a,b\ngär,tür\n")).expect("write");
    let (table, dialect) = read_csv_file(&path).expect("parse");
    assert_eq!(dialect.encoding, CsvEncoding::Utf16Le);
    assert_eq!(dialect.delimiter, b',');
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.rows, vec![vec!["gär".to_owned(), "tür".to_owned()]]);
}

#[test]
fn a_utf16be_bom_source_decodes_to_the_same_table() {
    let path = scratch("utf16be.csv");
    std::fs::write(&path, utf16be("\u{feff}a,b\nx,y\n")).expect("write");
    let (table, dialect) = read_csv_file(&path).expect("parse");
    assert_eq!(dialect.encoding, CsvEncoding::Utf16Be);
    assert_eq!(dialect.delimiter, b',');
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.rows, vec![vec!["x".to_owned(), "y".to_owned()]]);
}

#[test]
fn a_windows1252_semicolon_source_imports_with_its_text() {
    // A German Excel export: semicolon separator, Windows-1252 umlauts.
    let path = scratch("cp1252.csv");
    std::fs::write(&path, b"a;b\ng\xe4r;tor\n").expect("write");
    let (table, dialect) = read_csv_file(&path).expect("parse");
    assert_eq!(dialect.encoding, CsvEncoding::Windows1252);
    assert_eq!(dialect.delimiter, b';');
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.rows, vec![vec!["gär".to_owned(), "tor".to_owned()]]);
}

#[test]
fn a_tab_separated_source_sniffs_the_tab() {
    let path = scratch("tsv.csv");
    std::fs::write(&path, b"a\tb\n1\tx\n").expect("write");
    let (table, dialect) = read_csv_file(&path).expect("parse");
    assert_eq!(dialect.delimiter, b'\t');
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.rows, vec![vec!["1".to_owned(), "x".to_owned()]]);
}

#[test]
fn an_undefined_windows1252_byte_fails_precisely() {
    // 0x81 is one of the five bytes Windows-1252 never defines — the
    // decode refuses instead of silently losing the byte.
    let path = scratch("undefined.csv");
    std::fs::write(&path, b"a,b\nx,\x81\n").expect("write");
    let error = read_csv_file(&path).expect_err("the undefined byte must fail");
    assert!(
        error.contains("0x81") && error.contains("Windows-1252"),
        "the refusal names the byte and the encoding: {error}"
    );
}

#[test]
fn a_broken_utf16_stream_fails_precisely() {
    let odd = scratch("odd.csv");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend_from_slice(b"a\0b");
    std::fs::write(&odd, bytes).expect("write");
    let error = read_csv_file(&odd).expect_err("an odd byte count cannot decode");
    assert!(
        error.contains("UTF-16"),
        "the refusal names the encoding: {error}"
    );

    let surrogate = scratch("surrogate.csv");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend_from_slice(&[b'a', 0x00, 0x00, 0xD8, b'b', 0x00]);
    std::fs::write(&surrogate, bytes).expect("write");
    let error = read_csv_file(&surrogate).expect_err("a lone surrogate cannot decode");
    assert!(
        error.contains("UTF-16"),
        "the refusal names the encoding: {error}"
    );
}

#[test]
fn the_sniff_survives_quoted_commas_and_prefers_the_consistent_delimiter() {
    let comma_quoted = scratch("quoted.csv");
    std::fs::write(&comma_quoted, b"a,b\n\"x,y\",z\n").expect("write");
    let (_, dialect) = read_csv_file(&comma_quoted).expect("parse");
    assert_eq!(
        dialect.delimiter, b',',
        "quotes keep the embedded commas out of the count"
    );

    let semicolon = scratch("mixed.csv");
    std::fs::write(&semicolon, b"a;b\n\"x,y\";z\n").expect("write");
    let (table, dialect) = read_csv_file(&semicolon).expect("parse");
    assert_eq!(
        dialect.delimiter, b';',
        "the consistent semicolon count wins over the ragged comma count"
    );
    assert_eq!(table.rows, vec![vec!["x,y".to_owned(), "z".to_owned()]]);
}

#[test]
fn a_single_column_source_stays_on_the_comma_default() {
    let path = scratch("single.csv");
    std::fs::write(&path, b"a\n1\n2\n").expect("write");
    let (table, dialect) = read_csv_file(&path).expect("parse");
    assert_eq!(dialect.delimiter, b',');
    assert_eq!(table.fields.len(), 1);
    assert_eq!(table.rows.len(), 2);
}
