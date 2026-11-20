# PROGRESS_ARCHIVE.p90.md — Shard (Phase 90, append-only)

> Created 2026-11-20T17:57 — home for the detail block of Phase 90 on completion.
> Contains: Phase 90 — Publishing and CI hygiene. Header is write-once — never edited.

---

### Phase 90 — Publishing and CI hygiene

**Scope (Steps):**

- **90.1 Per-crate READMEs for the ten published crates with cargo package --list verification and the PUBLISHING checklist (L44, Medium gate)**
- **90.2 Ubuntu oracle job: bounded apt-get retry with backoff against transient DNS failures, persistent failures stay red (L40, High gate)**
- **90.3 Documentation and phase gate (L40, L44)**
**Verification:** Verification: docs_validate clean; cargo package --list per published crate shows the own README.md beside the three license files with no stray files and cargo fmt --check clean (90.1); the retry loop proven with apt-get PATH stubs in Git Bash (transient heals on attempt 3 with exit 0, persistent ends exit 1) and the workflow yaml parses (90.2); the phase's commits pushed to main and CI run 44777013803 fully green — the ubuntu oracle job's renamed step ran and logged "mdbtools install: attempt 1 of 3" / "installed on attempt 1" (90.2 acceptance: attempts visible in the log).
