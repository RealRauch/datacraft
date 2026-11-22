//! Per-session capabilities of the Jet/ACE adapter (phase 30, item M17;
//! ADR-7), split from `port_adapter.rs` to keep every file under the size
//! gate.
//!
//! The reference set is `full`, then the read-only shapes subtract:
//! Jet3 sessions, Access-locked sessions and sources whose protection
//! compact cannot reproduce (step 95.2, M126).

use datacraft_core::port::{DriverCapabilities, Support};

use crate::types::FormatVersion;

use super::Jet4Port;

pub(crate) fn capabilities(port: &Jet4Port) -> DriverCapabilities {
    let mut caps = DriverCapabilities::full();
    caps.atomic_batch = true; // batches apply in one writer session
    caps.passwords = true;
    caps.relationships = Support::Yes;
    caps.autonumber = Support::Yes; // LongInteger; ReplicationId answers Unsupported
    caps.compact = Support::Yes;
    // Compact refuses protection it cannot reproduce: a password- or
    // page-protected session and an unreadable protection state report
    // `No`, as do the read-only shapes below.
    let protected = port.reader.as_ref().is_some_and(|reader| {
        reader
            .header_info()
            .is_none_or(|info| info.password_protected || info.db_key != 0)
    });
    if protected {
        caps.compact = Support::No;
    }
    if port.opened_format() == Some(FormatVersion::Jet3) {
        caps.write_rows = false;
        caps.create_table = false;
        caps.drop_table = false;
        caps.alter_table = false;
        caps.atomic_batch = false;
        caps.autonumber = Support::No;
        caps.compact = Support::No;
    }
    if port.lock_file.is_some() {
        // A sibling Access lock file means Access may hold the database
        // open — read-only co-existence (step 53.1, item M59), the Jet3
        // read-only shape. DataCraft never creates or deletes lock files;
        // closing Access and removing the file restores the write session
        // on the next open.
        caps.write_rows = false;
        caps.create_table = false;
        caps.drop_table = false;
        caps.alter_table = false;
        caps.atomic_batch = false;
        caps.autonumber = Support::No;
        caps.compact = Support::No;
    }
    caps
}
