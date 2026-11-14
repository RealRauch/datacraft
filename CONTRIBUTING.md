# Contributing to DataCraft

> The binding working agreement is [`AGENTS.md`](AGENTS.md); the method is
> [`docs/PLAYBOOK.md`](docs/PLAYBOOK.md) and the review checklist is
> [`docs/LESSONS.md`](docs/LESSONS.md). This file is the practical entry:
> what to run, what has to stay green, and where the detail lives.

## Before you start

1. Read `AGENTS.md`, `docs/PLAYBOOK.md` and `docs/LESSONS.md`.
2. Pick work from [`BACKLOG.md`](BACKLOG.md) by priority, or the next open
   step of the active phase in [`PROGRESS.md`](PROGRESS.md).
3. Plan → package → implement. Implementation starts only after the human
   releases the step or phase; every step derives from an open backlog item.

No code without a step in `PROGRESS.md`; no known problem without an item
in `BACKLOG.md`.

## Test-first (RED → GREEN)

- Write the failing test first — a `test(scope): …` commit with the failure
  evidence — then the implementation in a `feat(scope): …` commit.
- Test names are snake_case behaviour descriptions. Assert on data and
  state, never on pixels; headless UI checks use the snapshot and the
  AccessKit tree.
- Exercise the real read/write path against committed fixtures. Mocks alone
  hide encoding and byte-order bugs.
- Fixtures are immutable evidence: `fixtures/manifest` pins their sha256.
  Never open them in Access or another writer-capable tool, and never
  regenerate them without the fixture gate.

## The canonical verification set

Run this before calling anything done:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --exclude datacraft-ui \
  --lcov --output-path lcov.info
```

The ≥ 95 % line gate is evaluated from the exported lcov — the same
explicit parse `ci.yml` runs — because the tool's embedded
`--fail-under-lines` check diverged from its own export (M64).
`datacraft-ui` is the registered coverage exception #1
([`docs/TESTING.md`](docs/TESTING.md) §3). A local `--fail-under-lines 95`
run is an approximation, not the gate.

Plus the file-size gate:

```bash
find . -name '*.rs' -not -path './target/*' -not -path './.git/*' \
  -exec wc -l {} \; | awk '$1 >= 500 {print $2}'
```

The Jet/ACE oracle suite needs `mdbtools` (CI: ubuntu and macOS) and is
`#[ignore]` by default:

```bash
DATACRAFT_ORACLE_MDB=… cargo test -p datacraft-jet --test oracle -- --ignored
```

A local skip reads as `ignored`; it is not a green oracle.

## What CI enforces

| Gate | Enforcement |
| --- | --- |
| Test coverage ≥ 95 % | lcov export parsed explicitly; `datacraft-ui` is the registered exception |
| Source files < 500 lines | the job fails on any `.rs` file ≥ 500 lines; split below the gate, not at it |
| No technical debt | tracked files carry none of the four banned markers defined in `docs/PLAYBOOK.md` §5 (append-only archives excluded) |
| Formatting and lints | `rustfmt --check`, `clippy -D warnings` |
| Cross-platform tests | ubuntu-latest, windows-latest, macos-latest, plus the packaged CLI smoke |
| Format oracle | libmdb cross-check of the decoded Jet4/ACE fixture tables and rows |
| Dependency boundaries | `.github/scripts/dependency_boundaries.py` — acyclic graph, composition roots only |
| Rustdoc | `cargo doc` with `-D warnings` |

## Commits and documentation

- Commit after every completed step; a phase ends with its gate commit.
- Messages are imperative, lowercase English with conventional prefixes
  (`feat:`, `test:`, `fix:`, `docs:`). Never amend — fix forward.
- Before committing: inspect `git status`, `git diff` and `git log`.
- Update `PROGRESS.md` after every step (🔄 before, ✅ after verification),
  `BACKLOG.md` for every found problem, and `CHANGELOG.md` at every
  completed phase.
- The README changes only when the user-visible surface changes (format
  path, client, platform, compatibility claim); feature detail belongs in
  `docs/FEATURES.md`, evidence in `docs/COMPATIBILITY.md`, gates here.
- Completed items and phases move unchanged into
  `docs/archive/` via the stepwell tools — never hand-edit those structures.

## Where things live

| Question | Document |
| --- | --- |
| Working method, gates, archive rules | [`docs/PLAYBOOK.md`](docs/PLAYBOOK.md) |
| Review checklist and recurring traps | [`docs/LESSONS.md`](docs/LESSONS.md) |
| System design, crate graph, ADRs | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| Test strategy, coverage exceptions | [`docs/TESTING.md`](docs/TESTING.md) |
| Jet/ACE binary-format knowledge | [`docs/FORMAT-JET.md`](docs/FORMAT-JET.md) |
| Compatibility claims and evidence | [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) |
| Feature inventory | [`docs/FEATURES.md`](docs/FEATURES.md) |
| Publishing, Semver, release checklist | [`docs/PUBLISHING.md`](docs/PUBLISHING.md) |
| Current work and next step | [`PROGRESS.md`](PROGRESS.md) |
| Open work items | [`BACKLOG.md`](BACKLOG.md) |
