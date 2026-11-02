//! Database-copy and password dispatch (step 27.2, item M14) — split from
//! `database.rs` to keep every file under the size gate.

use crate::core::failed;
use crate::event::Event;
use crate::port::DriverCapabilities;

impl crate::core::Core {
    /// Writes a copy of the open database (step 27.2, item M14). The copy
    /// is a new file; the source stays untouched and remains open.
    pub(crate) fn save_database_as(
        &mut self,
        path: std::path::PathBuf,
        password: Option<String>,
    ) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        // Capability gate (phase 30, M17; ADR-7): the driver declared no
        // password surface — refuse before the port, same Failed shape.
        if !db.capabilities.passwords {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "save a copy with database passwords",
            ));
        }
        let driver = db.driver.clone();
        if password.as_deref() == Some("") {
            return failed(format!(
                "unsupported: the {driver} driver needs a non-empty password — omit it for an \
             unencrypted copy"
            ));
        }
        let encrypted = password.is_some();
        match self.port.save_as(&path, password.as_deref()) {
            Ok(()) => vec![Event::DatabaseSavedAs { path, encrypted }],
            Err(e) => failed(e.to_string()),
        }
    }

    /// Replaces the open database's password in place (step 27.2, item
    /// M14); `None` removes the protection.
    pub(crate) fn change_password(&mut self, new_password: Option<String>) -> Vec<Event> {
        let Some(db) = self.state.database.as_ref() else {
            return failed("no database is open — open a database first");
        };
        if !db.capabilities.passwords {
            return failed(DriverCapabilities::unsupported(
                &db.driver,
                "change database passwords",
            ));
        }
        let driver = db.driver.clone();
        if new_password.as_deref() == Some("") {
            return failed(format!(
                "unsupported: the {driver} driver needs a non-empty password — omit it to remove \
             the protection"
            ));
        }
        let encrypted = new_password.is_some();
        match self.port.change_password(new_password.as_deref()) {
            Ok(()) => vec![Event::PasswordChanged { encrypted }],
            Err(e) => failed(e.to_string()),
        }
    }
}
