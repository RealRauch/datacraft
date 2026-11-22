//! Pre-open health facts for Jet/ACE (phase 95, step 95.1; item M126).
//!
//! The bounded, read-only sanity pass that runs once at open. The header
//! decode already refuses unusable files; this layer reports what a
//! readable file still carries: the page-0 header region facts and a
//! bounded page-type scan. A finding never blocks the open — the session
//! stays usable and every client surfaces the reason (the M91 pattern).
//!
//! The scan is deliberately shallow: one type byte per page, no row or
//! structure decode. Deep recovery belongs to the salvage reader, not to
//! the health check.

use datacraft_core::port::{SourceFact, SourceFacts};

use crate::jet::reader::Jet4TableReader;
use crate::types::FormatVersion;

/// Reports the file truth without touching the file (the reader already
/// holds the decrypted bytes).
pub(crate) fn inspect(reader: &Jet4TableReader) -> SourceFacts {
    let header = reader.header();
    let mut facts = vec![
        SourceFact {
            name: "jet_version".to_owned(),
            value: version_label(header.format).to_owned(),
        },
        SourceFact {
            name: "page_size".to_owned(),
            value: header.page_size.to_string(),
        },
        SourceFact {
            name: "page_count".to_owned(),
            value: header.page_count.to_string(),
        },
        SourceFact {
            name: "file_bytes".to_owned(),
            value: (header.page_size as u64 * header.page_count as u64).to_string(),
        },
    ];
    let health = if let Some(info) = reader.header_info() {
        facts.push(SourceFact {
            name: "code_page".to_owned(),
            value: info.code_page.to_string(),
        });
        facts.push(SourceFact {
            name: "encrypted".to_owned(),
            value: if info.db_key == 0 { "no" } else { "yes" }.to_owned(),
        });
        facts.push(SourceFact {
            name: "password".to_owned(),
            value: if info.password_protected { "yes" } else { "no" }.to_owned(),
        });
        page_scan(reader)
    } else {
        // Office-encrypted and writer-handoff readers carry no region
        // facts; the page scan still reports structural damage.
        page_scan(reader)
    };
    SourceFacts { facts, health }
}

/// Bounded page-type sanity: every page carries a type byte this build
/// knows or has observed in Access-written files. The documented set is
/// `0x00` unused … `0x05` usage map; `0x08`/`0x09` occur in the pinned
/// fixtures as Access-storage/long-value pages (FORMAT-JET "Pages"), and
/// DataCraft never reads them. The first three findings are reported; the
/// scan never decodes a page body.
fn page_scan(reader: &Jet4TableReader) -> Option<String> {
    const KNOWN_TYPES: [u8; 8] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09];
    let header = reader.header();
    let mut findings = Vec::new();
    for page in 1..header.page_count {
        let offset = page * header.page_size;
        let Some(&kind) = reader.file.get(offset) else {
            break;
        };
        if !KNOWN_TYPES.contains(&kind) {
            findings.push(format!("page {page} has unknown page type 0x{kind:02x}"));
            if findings.len() == 3 {
                break;
            }
        }
    }
    if findings.is_empty() {
        return None;
    }
    Some(format!(
        "{} (bounded scan of {} pages)",
        findings.join("; "),
        header.page_count
    ))
}

fn version_label(format: FormatVersion) -> &'static str {
    match format {
        FormatVersion::Jet3 => "Jet3",
        FormatVersion::Jet4 => "Jet4",
        FormatVersion::Ace => "ACE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_labels_are_stable() {
        assert_eq!(version_label(FormatVersion::Jet3), "Jet3");
        assert_eq!(version_label(FormatVersion::Jet4), "Jet4");
        assert_eq!(version_label(FormatVersion::Ace), "ACE");
    }
}
