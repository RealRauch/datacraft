# LESSONS.md — Review checklist (DataCraft adaptation)

> Project-specific adaptation of the canonical `stepwell-tool/docs/LESSONS.md`:
> only rules that apply to DataCraft (Rust desktop application parsing untrusted
> binary Jet/ACE database files). Upstream evidence references point to the
> origin project's archive and are omitted here; the full history lives
> upstream. **Usage:** before every review, before every phase completion, and
> when touching the parser, the query engine or storage backends.

## A. Tests that find something

1. **Tests must not mirror their own implementation.** "Smooth" fixtures confirm the code instead of checking it. The binary fixtures must include **realistic/evil** variants: truncated files, flipped magic numbers, zero-length tables, hostile column names, mixed collations, oversized memos.
2. **Secure contracts at the test seam, from both sides.** Where two layers share a convention (e.g. the `StoragePort` seam vs. a driver backend, UI state vs. headless command API), there must be a test on **both** sides with a **shared** fixture source — otherwise one side "guesses" the other.
3. **In-memory mocks hide real-world behavior.** A fake storage backend that never touches the filesystem looks correct in tests and breaks on real files (encoding, byte order, sparse pages). Always exercise the real read/write path against committed fixture databases.
4. **Silent error swallowing is diagnostic poison.** No bare `unwrap()`/`expect()` or ignored `Result` on fallible parse paths — a malformed file must produce a precise, locatable error, never "mysteriously does not work".

## B. Injection & dynamic queries

5. **Parameterized values do not protect identifiers.** Dynamic SQL (ORDER BY, object names interpolated from UI/QBE input) needs a **whitelist of known columns/tables** — otherwise reject. Relevant from Phase 7 (QBE builder) on: every identifier that reaches the engine passes the schema whitelist.

## C. Build & process

6. **Deterministic values or fail hard.** Anything written into the database file (engine version strings, creation timestamps, replication metadata) must be deterministic per input state OR fail loudly — silent fallbacks break bit-for-bit reproducibility of the write path.
7. **Security self-review before "done".** Deliberately walk the attack surface: what can a **crafted .accdb** do to the parser? A file claiming a 4 GB size? A self-referencing page? The regular suite mostly checks happy paths — the fuzzer and the libmdb oracle catch the rest.
8. **Mechanical mass replacements with placeholders are dangerous.** Replace only with exact, unique strings; placeholders must never collide with real content; always run a diff/anomaly check afterwards.
9. **Structure operations belong to the tool.** Status maintenance in the progress table, detail blocks, Done Index and archive changes always run through the stepwell tools (`progress_update`, `archive_item`, …) — manual edits to these structures create exactly the drift the tools prevent. Only the prose the tools deliberately do not manage (goal/acceptance texts, item content, new backlog items) is edited directly in the files.
10. **Measure exit codes in the host language.** PowerShell `$LASTEXITCODE`, POSIX `$?`/`$status` — never cmd `%ERRORLEVEL%` expansion after `&`. CI and local scripts use Git Bash or Rust assertions, not cmd.

## D. Preventive project rules (no upstream evidence yet — derived from project risk analysis)

11. **libmdb touches test code only.** The FFI/oracle lives behind `#[cfg(test)]`/feature gates; `unsafe` is confined to the oracle module; release builds never link it. Review every PR mentioning `mdbtools` or `unsafe` against this rule.
12. **No copied bytes.** Clean-room discipline: format knowledge flows from public specifications and our own observations only — never from MDB Tools source code. A review comment quoting MDB Tools internals is a High-class finding (content gate).
13. **Round-trip before write features.** Any writer change proves itself with decode(encode(decode(x))) == decode(x) over the fixture set before the feature is considered done.
14. **Gate the committed tree, not a dirty working tree.** `cargo fmt -- --check` (and every other working-tree gate) validates the current checkout — a formatting fix left uncommitted lets the gates pass locally while `HEAD` fails on CI (2026-11-08: the `writes.rs` import order was fixed by a `cargo fmt` run, re-broken by a manual edit, re-fixed by fmt — and the sorted file sat uncommitted while `--check` stayed green). Sequence discipline: finish edits → `cargo fmt --all` → stage and commit → run the canonical set; a `git status` line that survives a gate run unexplained is a finding.
15. **Split below the gate, not at it.** The 500-line file-size gate is absolute; a file riding the 480-499 band turns the next feature touch on it into a forced module split mid-step — phases 48 (`datasheet/trailing.rs`), 62 (`database/rows.rs`) and 63 (`forms/controls.rs`, `command/contexts.rs`) each carried unplanned splits. A released step that touches a band file splits it within that step; the live band list is tracked in `BACKLOG.md` (L30) and refreshed when the band grows.
