# AGENTS.md — Working agreement for AI agents

> Binding for every agent that touches this repository. This file is the short
> entry point; the authoritative method lives in
> [`docs/PLAYBOOK.md`](docs/PLAYBOOK.md) and [`docs/LESSONS.md`](docs/LESSONS.md).
> Read those two first — they win over this summary when they disagree.
>
> All project artifacts are **English** (code, comments, docs, tests, commit
> messages). Conversation with the human may be German.

## 1. What this project is

DataCraft is a clean-room, pure-Rust reimplementation of the classic Microsoft
Access workflow: a desktop shell (`eframe`/`egui`), a headless command bus, a
JSON-RPC/MCP endpoint and a CLI — all speaking the same `Command` vocabulary.
It reads Jet3/Jet4 `.mdb`, ACE `.accdb` and SQLite, and writes verified Jet4
table/row paths. No COM, no OLE DB, no `winapi`, no async runtime.

Workspace crates (dependency direction is review-enforced, never cyclic):

```text
datacraft-ui  ──► datacraft-core          (egui shell + datacraft-mcp binary)
datacraft-cli ──► datacraft-core          (headless CLI binary)
datacraft-storage ──► datacraft-core      (Jet/ACE + SQLite backends)
datacraft-core ──► (nothing internal)
```

Binary targets are the only composition roots allowed to name
`datacraft-storage` (`crates/*/src/main.rs`, `crates/datacraft-ui/src/bin/*`).
Library and UI code reaches storage exclusively through core commands.

## 2. Start every session in this order

1. Read `docs/PLAYBOOK.md` and `docs/LESSONS.md`.
2. Read `PROGRESS.md` — identify the next open step of the active phase and
   mark it 🔄 before starting.
3. Scan `BACKLOG.md` for open items and blockers.
4. Only then read the current step scope. Read files in parallel, never
   sequentially when independent.

Do not implement from chat intuition: **plan → package → implement**, and every
step derives from at least one open `BACKLOG.md` item.

## 3. Release gates (binding)

- Agents may **plan and package** freely; implementation starts only after the
  human explicitly releases the step/phase.
- Content risk classes:
  - **Low** — Rust source/test edits inside the released step scope: autonomous.
  - **Medium** — `Cargo.toml`/dependency changes, build scripts, new fixture
    databases: pause for per-occurrence release.
  - **High** — `.github/workflows/*`, deleting existing tests, license files,
    storage/clean-room decisions (bit-for-bit behaviour, FFI policy): explicit
    human gate.
- **Inline-fix lane** (only exception to the step boundary): a bug found inside
  the running step may be fixed immediately if it is within the step's code
  scope, ≤ ~10 lines, and Low-class only. Then add a retrospective `F` item to
  `BACKLOG.md` and archive it with the commit hash.

## 4. Quality gates — non-negotiable

Run the canonical verification set before calling anything done:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --exclude datacraft-ui \
  --lcov --output-path lcov.info --fail-under-lines 95
```

plus the file-size gate:

```bash
find . -name '*.rs' -not -path './target/*' -not -path './.git/*' \
  -exec wc -l {} \; | awk '$1 >= 500 {print $2}'
```

Hard rules (CI-enforced):

- Line coverage ≥ 95 % (CI ubuntu-latest is the binding gate; a local Windows
  `llvm-cov` number is not comparable and never a license to lower the bar).
- Every `.rs` file stays **below 500 lines** — split modules instead.
- No technical-debt markers anywhere in tracked files — the four banned tokens
  are defined in `docs/PLAYBOOK.md` §5 and scanned by the CI hygiene job. Code
  is final or not committed; known gaps become `BACKLOG.md` items.
- Coverage exclusions are only valid when registered in `docs/TESTING.md` §3.
  Unregistered exclusions are a High-class review finding.
- Measure exit codes in the host language (`$?` in Git Bash). Never read `$?`
  after a pipe.

## 5. Test-first (RED → GREEN)

- Write the failing tests first (`test(scope): …` commit with failure
  evidence), then the implementation (`feat(scope): …`).
- Test names are snake_case behaviour descriptions.
- Assert on data/state, never on pixels. Headless UI checks assert on
  `UiStateSnapshot` JSON.
- Exercise the real read/write path against committed fixtures; mocks alone
  hide encoding/byte-order bugs.
- Fixtures are **immutable evidence** (`fixtures/manifest` pins sha256). Never
  open them in Access or any writer-capable tool; manual experiments happen on
  a copy.
- Parser tests include realistic/evil inputs: truncated files, flipped magic,
  self-referencing pages, hostile names, oversized values.

## 6. Architecture constraints

- Every user-visible action is exactly one `Command` dispatched through
  `datacraft-core`. The GUI is not privileged: ribbon click and MCP call must
  produce identical state transitions.
- `datacraft-ui` is rendering glue only. Layout math, state and decisions live
  in core (the coverage exception is crate-scoped, not line-scoped).
- UI strings go through `t()`/`tr()` in `crates/datacraft-ui/src/i18n.rs`; the
  English source string is the key, the German catalog resolves it, unknown
  keys fall back to English. Language stays UI-owned — never in core, sidecar
  or wire.
- No async runtime. Background work uses `std::thread` + channels behind core
  ports.
- `unsafe_code = "forbid"` workspace-wide. libmdb/MDB Tools is a **test-time
  CLI oracle only** (`DATACRAFT_ORACLE_MDB`, `mdb-export`), never linked or
  shipped.
- Clean-room is binding: format knowledge only from public specifications and
  own black-box observation. Never copy or quote MDB Tools source.
- Identifiers that reach the query engine pass the schema whitelist.
  Parameterized values do not protect identifiers.
- Errors on malformed input are precise and locatable. No `unwrap`/`expect` or
  ignored `Result` on parse paths.
- Values written into a database file are deterministic per input state or the
  operation fails loudly.
- Sidecar vs native (ADR-1): forms, sidecar queries and reports live in
  `<database>.datacraft`; native Access queries/macros/VBA are read-only.
  Opening a database must not modify it.
- No macro or VBA execution (ADR-2). Extraction, display and static analysis
  only.

## 7. Documentation and commit discipline

- `README.md`, `PROGRESS.md`, `BACKLOG.md` are living documents. Update the
  step status (🔄 before, ✅ after verification), README on every behaviour
  change, and BACKLOG on every found bug/risk/gap.
- **No code without a step in `PROGRESS.md`; no known problem without an item
  in `BACKLOG.md`.**
- Completed items/phases move unchanged into
  `docs/archive/BACKLOG_ARCHIVE.md` / `docs/archive/PROGRESS_ARCHIVE.md`;
  archives are append-only. Use the stepwell tools for status tables, detail
  blocks, Done Index and archive moves — do not hand-edit those structures
  (LESSONS #9).
- Item IDs: `K/H/M/L` priority series, `T` test gaps, `F` inline-fix
  retrospectives; consecutive, never reused. Timestamps ISO 8601
  `YYYY-MM-DDTHH:mm`.
- Commit after every completed step, mandatory after a phase. Messages are
  imperative, lowercase English, conventional prefixes (`feat:`, `test:`,
  `fix:`, `docs:`). Never amend; fix forward.
- Before committing: inspect `git status`, `git diff`, `git log --oneline -10`;
  stage only intended files.

## 8. Environment notes

- Platform: Windows, shell is **Git Bash**.
- Toolchain is pinned in `rust-toolchain.toml` (Rust 1.98.1); MSRV is 1.98.
- Line endings are normalized to LF via `.gitattributes`; binaries
  (`*.png`, `*.mdb`, `*.accdb`, …) stay binary.
- `lcov.info` is generated and git-ignored; `.zcode/` holds local session
  artifacts and is never committed.
- The oracle suite needs `mdbtools` (available on Ubuntu/macOS CI, not on
  Windows): `DATACRAFT_ORACLE_MDB=… cargo test -p datacraft-storage --test oracle`.

## 9. Where to look

| Question | Document |
| --- | --- |
| Working method, gates, archive rules | [`docs/PLAYBOOK.md`](docs/PLAYBOOK.md) |
| Review checklist and recurring traps | [`docs/LESSONS.md`](docs/LESSONS.md) |
| System design, crate graph, ADRs | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| Test strategy, coverage exceptions | [`docs/TESTING.md`](docs/TESTING.md) |
| Jet/ACE binary-format knowledge | [`docs/FORMAT-JET.md`](docs/FORMAT-JET.md) |
| Compatibility rings and scope decisions | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Fixture provenance | [`docs/FIXTURE-SOURCES.md`](docs/FIXTURE-SOURCES.md) |
| Current work and next step | [`PROGRESS.md`](PROGRESS.md) |
| Open work items | [`BACKLOG.md`](BACKLOG.md) |

## 10. Never

- Never edit the append-only archives.
- Never modify or regenerate fixture files or their hashes.
- Never open fixtures in Microsoft Access or another writer-capable tool.
- Never weaken or skip a quality gate, delete tests, or add an unregistered
  coverage exclusion.
- Never add a dependency, change `Cargo.toml`, or touch CI workflows without
  the matching release gate.
- Never put UI logic, locale or filesystem paths into `datacraft-core`.
- Never claim compatibility beyond the verified matrix in `README.md`.
