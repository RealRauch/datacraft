//! DataCraft storage — `StorageEngine` trait boundary and the pure-Rust
//! Jet/ACE backends (clean-room; `libmdb` serves as test-time oracle only).
//!
//! Phase 2 fills this crate; see `docs/ARCHITECTURE.md` §6. At this point the
//! crate is scaffolding only (structure-file exception, `docs/PLAYBOOK.md` §6).

#[cfg(test)]
mod tests {
    /// Scaffolding smoke test: the crate links and the test harness runs.
    #[test]
    fn storage_crate_is_loadable() {}
}
