# PROGRESS_ARCHIVE.p79.md — Shard (Phase 79, append-only)

> Created 2026-11-14T17:24 — home for the detail block of Phase 79 on completion.
> Contains: Phase 79 — README Audience Split. Header is write-once — never edited.

---

### Phase 79 — README Audience Split

**Scope (Steps):**

- **79.1 Audience contract, README outline and the growth-rule amendment (PLAYBOOK §2, AGENTS §7)**
- **79.2 docs/FEATURES.md — move the feature inventory; README keeps a short overview**
- **79.3 docs/COMPATIBILITY.md — move the full evidence matrix; README keeps the dated short matrix**
- **79.4 User entry — current screenshot plus a release-download and install path**
- **79.5 CONTRIBUTING.md — quality gates and workflow; README keeps a one-line gate statement**
- **79.6 Retire the README roadmap table; carry M95 into the ARCHITECTURE §13 mapping**
- **79.7 Final pass — links, word budget, policy pointers, docs_validate and the phase gate**

**Goal:** the README becomes a user entry — pitch, screenshot, both
install paths, a compact "can it open my file?" matrix and the licence —
at ≤ ~180 lines / ≤ ~1,200 words. Every body of detail moves to its own
home (`docs/FEATURES.md`, `docs/COMPATIBILITY.md`, `CONTRIBUTING.md`,
existing `docs/ROADMAP.md`/`PROGRESS.md`), and the binding rule that
grew the file is amended so the next phase gate cannot regrow it.

- **79.1 — audience contract and outline.** Write the three-reader
  contract and the target README section map (below); define which
  changes are *user-visible surface* and which belong in `CHANGELOG.md`/
  `PROGRESS.md` only. The binding wording in PLAYBOOK §2 and AGENTS §7
  lands with 79.7, when the destination docs exist — no intermediate
  pointer to a missing file. Documentation-only, Low.

**Audience contract:**

- **End user (no Rust):** what the tool is, one screenshot, where to
  download it, how to open an existing database, what is verified — and
  nothing about step numbers, test files or CI lanes.
- **Crate consumer (`cargo add`):** where the libraries live and which
  crate does what, MSRV and licence, pointer to docs.rs. Per-crate
  landing pages are L44 (separate Medium gate), not this phase.
- **Contributor/agent:** where the method lives (`docs/PLAYBOOK.md`,
  `docs/LESSONS.md`), the gates (`CONTRIBUTING.md`), architecture and
  format knowledge (`docs/`), current work (`PROGRESS.md`/`BACKLOG.md`).
  The README links these; it does not inline them.

**User-visible surface — the only README trigger:** a new or changed
read/write format path, a new or changed client (GUI/CLI/MCP), a new or
changed platform, a changed compatibility claim or known limit, or a
shipped workflow the user can act on. Everything else — internal
commands, tests, refactors, step completions — updates `CHANGELOG.md`,
`PROGRESS.md`, `docs/ROADMAP.md` or `docs/TESTING.md` instead.

**Outline (target README, ~180 lines):**

1. hero: icon, one-sentence pitch, badges, section links;
2. a current screenshot of the shell, one caption line;
3. Download & install: the three release archives plus the cargo path;
4. What works: 8–10 bullets (formats read/write, QBE, forms/reports,
   SQL, MCP/CLI, migration tool);
5. "Can it open my file?": the compact matrix (format × read/write ×
   known limit), dated, linking `docs/COMPATIBILITY.md`;
6. Agents & clients: MCP/JSON-RPC and CLI one-liners;
7. Architecture: one paragraph plus the crate table;
8. Contributing: one gate line plus links to `CONTRIBUTING.md` and the
   playbook;
9. License and credits: unchanged, compact.
- **79.2 — `docs/FEATURES.md`.** Move the "What works today" table
  (2,936 words) with its phase/step provenance intact; README keeps an
  8–10 bullet overview. Claim-preservation check: every removed README
  claim maps to a FEATURES.md row (diff reviewed in the step note).
- **79.3 — `docs/COMPATIBILITY.md`.** Move the full evidence matrix
  (test files, oracle lanes, CI runs, known limits) into the new doc;
  README keeps a compact format × read/write matrix with the existing
  date stamp and a link. The README matrix stays the only compatibility
  claim (AGENTS §10); no row may be dropped or softened, only relocated.
  POLICY-text touch on AGENTS/PUBLISHING = Medium.
- **79.4 — user entry.** Capture a current screenshot of the shell (open
  fixture, light appearance) under `docs/images/` and reference it near
  the hero; add a Download/Install section linking the GitHub release
  page and the three archives, keeping `cargo run` as the developer path.
  If a fresh capture is not possible in this environment, reuse
  `docs/phase-20/after-light.png`, state its age and file the follow-up
  item — no silent stale visual.
- **79.5 — `CONTRIBUTING.md`.** Move the quality-gate table and the
  canonical verification set (fmt, clippy, tests, lcov parse, file-size,
  boundary guard, rustdoc, oracle) plus the test-first/STEPWELL/commit
  rules into the contributor doc; README keeps a one-line statement with
  a link. Cross-check the wording against `docs/TESTING.md` and CI.
- **79.6 — roadmap table retirement and M95.** Remove the 64-row roadmap
  table from the README (link `docs/ROADMAP.md` + `PROGRESS.md`); add
  phases 48–78 to the `docs/ARCHITECTURE.md` §13 mapping and archive M95
  through the stepwell tools. M96 stays open until the phase gate.
- **79.7 — final pass and gate.** Line/word budget, internal-link audit,
  policy-pointer check (AGENTS/PLAYBOOK/PUBLISHING name the new homes),
  `docs_validate`, and the canonical verification set. Documentation-only
  change: no CHANGELOG entry (no user-visible behaviour), no source edit.

Content classes: documentation only; binding policy text in 79.3/79.7 is
Medium and released with the phase; the screenshot is a new asset (not a
fixture, Low). No High-class file is touched.

**Acceptance:** README ≤ ~180 lines and ≤ ~1,200 words; it opens with the
pitch, a current screenshot and both install paths (release archive and
cargo); dev-only content lives in `docs/FEATURES.md`,
`docs/COMPATIBILITY.md`, `CONTRIBUTING.md` and `docs/ROADMAP.md`; the
README matrix remains the only compatibility claim and no claim is lost or
weakened; every delivered phase appears in ARCHITECTURE §13; all internal
links resolve; `docs_validate` clean; canonical set green.

**Verification:** internal link audit over the touched docs (one-off
checker), `docs_validate` clean, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, the lcov export parsed at ≥ 95 % lines, and the
file-size gate — all green.
**Verification:** Link audit and docs_validate clean; fmt, clippy and workspace tests green; file-size gate clean; no .rs/.toml differs from the v0.2.0 release commit (coverage structurally unchanged). README is 1,948 words / 245 lines (prose without the reference tables 1,338) — the 1,200/180 target is missed; the residual is the 610-word compatibility matrix plus the crate table, and every compatibility claim is intact.
