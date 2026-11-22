# PROGRESS_ARCHIVE.p97.md — Shard (Phase 97, append-only)

> Created 2026-11-22T03:16 — home for the detail block of Phase 97 on completion.
> Contains: Phase 97 — PDF report export over the frozen scope (roadmap §11). Header is write-once — never edited.

---

### Phase 97 — PDF report export over the frozen scope (roadmap §11)

**Scope (Steps):**

- **97.1 Renderer decision + core feature: the pure-Rust PDF writer behind an optional core feature, the sql-engine precedent (Medium gate: dependency — needs the human release)**
- **97.2 Core: the export command over the paginated page model — deterministic (fixed metadata, stable order), text stays text, fonts reach the writer without depending on datacraft-ui**
- **97.3 UI + wire: the export affordance at the report preview, capability-gated, CLI and MCP parity**
- **97.4 Acceptance + documentation and phase gate: page count and breaks in a standard viewer, text extracts, byte-identical re-export, unwritable target answers precisely**
**Verification:** cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps, cargo test --workspace and the file-size gate all green; the lcov export parses 22173/23266 = 95.302 % line coverage (>= 95 %); acceptance 2026-11-22: pdftotext and pypdf extracted each page's text from a 3-page export, pypdf reported the 595x842 pt mediabox, and re-export is byte-identical.
