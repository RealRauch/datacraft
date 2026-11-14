# BACKLOG_ARCHIVE.p79.md — Shard (Phase 79, append-only)

> Created 2026-11-14T17:14 — home for item blocks archived in the context of Phase 79 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 79. Header is write-once — never edited.

---

### [x] M95 — ARCHITECTURE §13 phase mapping stops at phase 47 — 🟢
- **Location:** `docs/ARCHITECTURE.md` §13 phase mapping (last row: phase 47).
- **Problem:** The table claims to map the delivered phases, but phases 48–78 are missing; `PROGRESS.md`, `CHANGELOG.md` and `docs/ROADMAP.md` are current. A reader of the architecture blueprint sees a stale picture. The README half of the original finding (a 64-row roadmap table ending at phase 64) is retired with the README restructure — no table is carried forward there.
- **Approach:** add the missing rows from `CHANGELOG.md`/`PROGRESS.md` at the next documentation pass (one verified line per phase, grouping where the existing table already groups); no code change. Rides Phase 79 step 79.6 with the README roadmap retirement.
- **Acceptance:** every delivered phase in `PROGRESS.md` appears with its one-line deliver in ARCHITECTURE §13; `docs_validate` clean.
- **Steps:** Phase 79 step 79.6.
- **Done:** 79.6 e914d03

## ✅ Done Index

> One-liner per completed item (details above).

- M95 — ARCHITECTURE §13 phase mapping stops at phase 47 — done (79.6 e914d03)
