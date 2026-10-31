# HANDOFF — Session break 2026-11-01

> Context window of the previous session was exhausted. This file carries the
> exact resume state. The new session starts per PLAYBOOK §0.8: read
> `docs/PLAYBOOK.md`, `docs/LESSONS.md`, `PROGRESS.md`, `BACKLOG.md` — then
> this file. Delete this file after T2 is resolved (it is session-specific).

## Where we are

- **Released:** Phases 3–9 (human, 2026-10-31, blanket release with the
  instruction to raise open questions BEFORE implementation and then work
  autonomously — see the decision defaults below).
- **Phase 2 status:** steps 2.1–2.3 ✅; **step 2.4 ⛔ blocked on BACKLOG T2**
  (real-row tail semantics); step 2.5 (gate review) pending. Phase 2 is
  therefore still open.
- **Branch model:** everything merged to `main` (phase-checkpoint merge
  `07cdff7`); `feat/jet-storage-core` and `design/datacraft-app-icon` deleted
  after merge (binding phase-end merge rule now in PLAYBOOK §4). Work
  continues on `main`.
- **CI:** fully green on main as of the T1 fix; remote
  `https://github.com/RealRauch/datacraft.git` (private). The 2.4/T2 work was
  pushed to the feature branch and merged — verify CI on main after resuming.

## T2 — the immediate blocker (BACKLOG T2, 🔵)

Real Jet4 rows disagree with the synthetic tail model. Evidence gathered on
the pinned fixture (`fixtures/testV2000.mdb`, jackcess `be0a2139`, sha256
`126bad49…a5`): catalog TDEF decodes perfectly (17 columns — Connect=Memo j=3,
Name=Text j=0, Type=Int2 fixed, Flags=LongInt …), rows parse (num_cols=17,
var_len=11), fixed values plausible, BUT null mask reads `[ff 00 00]` where
descriptor-order expects `[5c 28 01]`, and var_table entries read 50/52 while
eod=46. Hypotheses + all dumps: `docs/FORMAT-JET.md` → "Open: real-row tail
semantics (BACKLOG T2)". Oracle harness: `crates/datacraft-storage/tests/
oracle.rs`, env `DATACRAFT_ORACLE_MDB=fixtures/testV2000.mdb` (our side runs
without mdb-export; value comparison needs the CLI — CI oracle job planned but
**not yet added to ci.yml**).

**Suggested next move for T2:** hex-compare one known-content MSysObjects row
against mdbtools' documented Jet4 tail order (var_table may be FORWARD in
Jet4, not reverse — HACKING's reverse note is in the Jet3 section; the mask
may cover max_cols or sit elsewhere). Verify with the oracle, fix
`RowLayout::parse`, keep `tests/support` builders byte-exact with reality,
then finish 2.4 + 2.5 and continue with Phase 3.

## Decision defaults (ratified, veto-able — see session log 2026-10-31)

1. Jet/ACE **writer** = own workstream after Phase 4 (item H9).
2. **Oracle fixtures**: pinned jackcess test files (test data, not code).
3. Encrypted DBs → precise `Unsupported`.
4. Jet3 read deferred (item L1).
5. **ADR-1**: design layer persists as versioned `.datacraft` sidecar (serde)
   until the ACE writer exists.
6. UI font: egui default until Outfit lands as a tracked asset.
7. Query engine: in-memory predicates over StorageEngine rows (no SQL
   parser); identifier whitelist per LESSONS #5.
8. MCP transport: stdio JSON-RPC 2.0.
9. CLI client = fourth command-bus client (item H8, step 9.4).

## Environment quirks (Windows, Git Bash)

- Toolchain pinned `1.98.1` (`rust-toolchain.toml`); `cargo-llvm-cov`
  installed locally; CI gate: `cargo llvm-cov --workspace --exclude
  datacraft-ui --fail-under-lines 95` (exception #1 documented in
  docs/TESTING.md §3 — `#[coverage(off)]` is unstable on stable rust).
- Python patch scripts beat shell heredocs (backtick/`\\u` escaping); watch
  rustfmt re-anchoring text replacements (re-grep after every `cargo fmt`).
- `#[cfg(test)]` does NOT apply in integration-test builds — runtime env-gate
  debug code instead.
- A parallel branding session may still write files (`.tmp-brand*`, assets/);
  never `git add -A` blindly — stage targeted paths.
- mdb-export is NOT installed locally (oracle value comparison runs in CI
  only); our-side oracle checks run locally with the env var set.

## Next steps (order)

1. Resolve T2 → finish 2.4 (oracle green incl. CI job) → 2.5 gate review →
   archive Phase 2.
2. Phase 3 — Data Model & Schema (add backlog items JIT per PLAYBOOK §0.3).
3. Phases 4–9 per PROGRESS.md; phase-end merges per PLAYBOOK §4.
4. GitHub CI: check the run on main after resuming (gh run list).
