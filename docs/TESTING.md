# TESTING.md — Test strategy & coverage policy

> Step 1.3 (item H2). Living document. Language: English (binding).
> The coverage exception registry below is **part of the quality gate**:
> an exception that is not listed here is a CI failure, not a style choice.

## 1. Canonical verification command set

Every step is verified with (all green, or the step is not done — PLAYBOOK §5):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --exclude datacraft-ui \
  --lcov --output-path lcov.info --fail-under-lines 95
```

plus the file-size gate (CI-side; local check with Git Bash):

```bash
find . -name '*.rs' -not -path './target/*' -not -path './.git/*' \
  -exec wc -l {} \; | awk '$1 >= 500 {print $2}'
```

(any output = violation). Exit codes are measured directly — never read from a
pipe position (`$?` after a pipe reports the *last* command, LESSONS #10).

## 2. Coverage policy

- Tool: `cargo-llvm-cov`, gate **≥ 95 % lines**, workspace-wide, enforced by CI.
- Rationale for line coverage over region/branch: stable, cheap, mechanical;
  branch intent is covered by explicit table-driven tests instead.
- **Platform note (2026-11-01, measured):** a local Windows `llvm-cov` run
  reports **92.81 %** lines for the Phase 16 commit (`8d5c1cb`) whose Linux CI
  coverage job passed. The cause was not isolated; the numbers are not
  comparable across hosts. The ubuntu-latest CI job is the binding gate — a
  local Windows number below 95 % is not a license to exclude files or lower
  the threshold, and every coverage exception still needs a registry row in
  §3.
- **Every exception must be registered below.** Unregistered exclusions are a
  High-class review finding (quality gate manipulation).

## 3. Coverage exception registry

| # | Scope | Mechanism | Justification | Rules | Revisit |
|---|-------|-----------|---------------|-------|---------|
| 1 | crate `datacraft-ui` | `--exclude datacraft-ui` on the llvm-cov invocation (CI + local) | The crate is eframe/egui rendering glue, but **no longer untested**: the `egui_kittest` suite (`crates/datacraft-ui/tests/ui_kittest.rs`, step 23.2/T5) drives the real widgets through AccessKit (ribbon, nav pane, datasheet, SQL window, backstage) and gates their semantic tree with committed `insta` snapshots. Line coverage stays excluded because `#[coverage(off)]` is unstable on the pinned toolchain and paint closures are not meaningfully line-covered without a window. | Rendering glue only — **all logic lives in `datacraft-core`** (covered). Logic found in `ui` (layout math, state, commands) is a review finding and moves to core. | When the coverage attribute stabilizes, or if eframe enables headless `App` execution. |
| 2 | `datacraft-cli/src/main.rs` (the bin target only) | none — the ~20-line composition root counts against the gate as uncovered | The bin's **behaviour** is covered by the real-subprocess suite `crates/datacraft-cli/tests/cli_process.rs` (step 23.1, T4): open/exec/non-interactive repl over real stdio plus the exit-code matrix and the stderr routing. The line-coverage exclusion remains because the bin only wires the real stdio handles into `run_split` and subprocess profiles are not merged on every host. **All logic lives in the `datacraft_cli` library and is covered.** | The bin stays a thin `main`; logic found in it is a review finding and moves to the library. | When subprocess profiles are merged reliably on all CI hosts, or the coverage attribute stabilizes. |

The exclusion is deliberately **crate-scoped, not line-scoped**: it can only
shrink, never hide, logic that wrongly migrated into the shell crate.

## 4. Test taxonomy

| Layer | Location | Purpose |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` in the same file | one behavior per test, colocated, fastest feedback |
| Integration | `crates/*/tests/*.rs` | cross-module/cross-crate contracts |
| Property | `proptest` in unit/integration modules | parser/round-trip invariants (decode∘encode∘decode, serde round-trips) |
| Golden file | `fixtures/*.mdb`/`*.accdb` (Phase 2) | byte-level compatibility against reference databases created with MDB Tools tooling only |
| Fixture pin integrity | `fixtures/manifest` + CI `fixtures` job | committed sha256 per fixture (`repo@commit` provenance, docs/FIXTURE-SOURCES.md); a drifted or hand-edited fixture fails the build |

**Fixture immutability rule (T3, 2026-11-01):** fixtures are immutable
evidence — never open them in Microsoft Access or any other writer-capable
Jet/ACE application (opening alone rewrites system pages and appends
pages). Manual byte experiments run on a copy; the provenance case study is
docs/FIXTURE-SOURCES.md §4. The two committed `testV2000` variants carry
this history: `testV2000.mdb` is the pristine jackcess pin (`repo@commit`),
`testV2000-access-touched.mdb` documents what real Access does on open.
| Oracle | `cfg(test)` libmdb backend (Phase 2) | strict schema/row cross-check of our decoder against libmdb |
| Headless E2E | `datacraft-core` command bus (Phase 9) | assert `UiStateSnapshot` JSON — never pixels |
| Headless UI | `datacraft-ui/tests/ui_kittest.rs` (T5) | real egui widgets driven by AccessKit labels/roles; committed `insta` snapshots of the semantic tree are the layout gate — DPI/font-independent, never pixel compares |

## 5. Conventions

- Test names: snake_case behavior descriptions, e.g.
  `object_id_json_roundtrip_preserves_identity`,
  `jet_header_rejects_truncated_file`.
- **Fixtures are realistic/evil** (LESSONS #1): truncated files, flipped magic
  numbers, zero-length tables, hostile names — never only happy paths.
- Errors on malformed input must be precise and locatable — no silent
  swallowing (`unwrap`/ignored `Result` on parse paths, LESSONS #4).
- RED→GREEN: new tests land in their own `test(scope): …` commit with failure
  evidence, then the implementation (`feat(scope): …`) — PLAYBOOK §4/§6.
- Structure files (manifests, scaffolding) may skip the RED step; the reason
  is noted in the commit (PLAYBOOK §6 exception).
