# PROGRESS_ARCHIVE.p94.md — Shard (Phase 94, append-only)

> Created 2026-11-22T00:23 — home for the detail block of Phase 94 on completion.
> Contains: Phase 94 — i18n bypass guard (T12). Header is write-once — never edited.

---

### Phase 94 — i18n bypass guard (T12)

**Scope (Steps):**

- **94.1 UI paint-site scan: t()-bypass guard over the UI crate, RED first (T12)**
- **94.2 CI hygiene job wiring for the guard (T12, High gate: needs the human release)**
- **94.3 Documentation and phase gate (T12)**
**Verification:** cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps && cargo test --workspace -j 2 && cargo llvm-cov --workspace --exclude datacraft-ui --lcov --output-path lcov.info — all green; the explicit lcov parse reads 95.565 % (>= 95); the file-size scan lists no file >= 500 lines; cargo test --locked -p datacraft-ui --lib i18n::bypass green (the hygiene job's exact step); the pushed head's CI run on main is the binding confirmation (checkpoint: 3b1d3ab)
