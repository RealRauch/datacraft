# PROGRESS_ARCHIVE.p89.md — Shard (Phase 89, append-only)

> Created 2026-11-19T19:59 — home for the detail block of Phase 89 on completion.
> Contains: Phase 89 — Decisions and design review. Header is write-once — never edited.

---

### Phase 89 — Decisions and design review

**Scope (Steps):**

- **89.1 Tool windows as task panes — decision record in docs/ROADMAP.md with go/no-go and a scoped follow-up if yes (M100)**
- **89.2 Native menu bar on macOS — decision record; if yes, existing commands only and the M99 shortcut model (L46)**
- **89.3 Low-power/RDP rendering mode — detection decision and the mode's measurable effects (L50)**
- **89.4 Design pass: the human's live density/typography/spacing review with per-finding items (L47)**
- **89.5 Documentation and phase gate (M100, L46, L50, L47)**
**Verification:** cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps; cargo test --workspace; cargo llvm-cov --workspace --exclude datacraft-ui --lcov --output-path lcov.info — all green; file-size gate empty; the first cargo test run hit corrupt target artifacts (only-metadata-stub std/alloc rmeta, E0463/E0433 in well-formed sources) — recompiled clean and green, sources untouched this phase; the ≥ 95 % line gate is binding on the CI ubuntu-latest run
