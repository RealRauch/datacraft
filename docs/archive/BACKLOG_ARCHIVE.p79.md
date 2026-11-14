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

---

### [x] M96 — README is 371 lines and development-focused — split it into a user entry plus feature, evidence and contributor docs — 🟡
- **Location:** `README.md` (371 lines, ~7,330 words; measured 2026-11-14). Weight: "What works today" 2,936 words, "Roadmap" 1,372 (64-row table, ends at phase 64), "Compatibility matrix" 1,092, "Architecture" 510, "Design & branding" 286, "Quality gates" 149 — against ~450 words of genuine user entry (pitch, quick start, license).
- **Problem:** One file serves four audiences at once. A desktop-app user finds no screenshot and no download/install path (the only entry is `cargo run -p datacraft-ui`), while the feature table, the 64-row roadmap and the quality-gate table duplicate `PROGRESS.md`, `docs/ROADMAP.md` and CI at step granularity and drift at every phase gate. All nine published crates inherit this product page through `readme.workspace = true`. Root cause is the "update README after every behaviour change" rule (PLAYBOOK §2, AGENTS §7): trimming the text alone regrows at the next gate.
- **Approach:** Phase 79 separates the readers and moves each body of detail to its own home: feature inventory → `docs/FEATURES.md`, full evidence matrix → `docs/COMPATIBILITY.md` (README keeps a date-stamped short matrix), gate detail → `CONTRIBUTING.md`, roadmap prose → existing `docs/ROADMAP.md`/`PROGRESS.md`; README gains a current screenshot, a release-download path and a short gate statement. AGENTS §7/§10, PLAYBOOK §2 and PUBLISHING §7 name the new homes so the growth rule stops. The README roadmap table is retired — M95's README half is absorbed here, its ARCHITECTURE §13 half stays open.
- **Steps:** Phase 79 (79.1 audience contract and outline; 79.2 `docs/FEATURES.md`; 79.3 `docs/COMPATIBILITY.md`; 79.4 screenshot + download/install entry; 79.5 `CONTRIBUTING.md`; 79.6 retire the roadmap table and reconcile M95/ARCHITECTURE §13; 79.7 final pass, policy alignment and gate).
- **Acceptance:** README ≤ ~180 lines and ≤ ~1,200 words; it opens with the pitch, a current screenshot and both install paths (release archive and cargo); dev-only content lives in `docs/FEATURES.md`, `docs/COMPATIBILITY.md`, `CONTRIBUTING.md` and `docs/ROADMAP.md`; the README matrix remains the only compatibility claim and no claim is lost or weakened; all internal links resolve; `docs_validate` clean.
- **Done:** 79.7 41926aa — readme 1,948 words / 245 lines (prose without reference tables 1,338); 1,200/180 budget missed, residual is the 610-word compatibility matrix, all claims intact

## ✅ Done Index

> One-liner per completed item (details above).

- M95 — ARCHITECTURE §13 phase mapping stops at phase 47 — done (79.6 e914d03)
- M96 — README is 371 lines and development-focused — split it into a user entry plus feature, evidence and contributor docs — done (79.7 41926aa — readme 1,948 words / 245 lines (prose without reference tables 1,338); 1,200/180 budget missed, residual is the 610-word compatibility matrix, all claims intact)
