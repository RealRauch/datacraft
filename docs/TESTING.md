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
  --lcov --output-path lcov.info
```

The ≥ 95 % line gate is evaluated from the exported lcov — the same
explicit parse `ci.yml` runs — because the tool's embedded
`--fail-under-lines` check diverged from its own export (run
41727473021, M64). The `datacraft-ui` exclusion above is the registered
coverage exception #1 (§3). A local `--fail-under-lines 95` run is an
approximation, not the gate.

plus the file-size gate (CI-side; local check with Git Bash):

```bash
find . -name '*.rs' -not -path './target/*' -not -path './.git/*' \
  -exec wc -l {} \; | awk '$1 >= 500 {print $2}'
```

(any output = violation). Exit codes are measured directly — never read from a
pipe position (`$?` after a pipe reports the *last* command, LESSONS #10).

**Features in the canonical set (phase 76, L37):** `datacraft-core`'s default
feature set is empty; the `sql-engine` switch forwards to
`datacraft-query/sql-engine`. `cargo test --workspace` still exercises the
engine because the three client roots (UI, CLI, MCP) enable the feature and
Cargo unifies it across the build. The three engine-dependent core test
targets (`sql_engine`, `sql_command`, `native_views`) declare
`required-features = ["sql-engine"]`, so `cargo test -p datacraft-core`
without the feature skips them instead of failing; CI's
`--all-features` run is the widest lane.

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
| 4 | defensive/unreachable branches in covered crates | none — the lines count against the gate as uncovered, accepted like #2/#3 | Exhaustiveness-defense match arms the public API cannot reach (e.g. `datacraft-expr/src/coerce.rs` negate/numeric_f64 arms for Currency/Decimal via parsed text — `parse_num_text` yields only Int/Float; `csv/src/infer.rs` empty-records early return — the caller rejects header-less files first), plus MSVC-vs-linux attribution artifacts that CI hits but Windows runs cannot (verified 2026-11-04: same cargo-llvm-cov 0.9.1, same pinned toolchain, same gate math — the residual local-vs-CI delta is target codegen region mapping, tracked by step 43.1/T9 which raised the local rate to ≥ 95%). | Lines become covered when a public path reaches them; an arm that stays unreachable forever may be simplified in review instead of excluded. | Never — informational; the binding gate stays CI ubuntu-latest. |
| 3 | `datacraft-mcp/src/main.rs` (the whole binary-only crate) | none — the ~11-line composition root counts against the gate as uncovered | The crate is a pure composition root (composed driver set + real stdio handles, no library, no logic of its own). Its **behaviour** is covered by the real-subprocess suite `crates/datacraft-mcp/tests/mcp_process.rs` (step 36.3, T7): inspect over real stdio, open-then-inspect sessions, the CSV directory through the registered fallback, the L9 failure message over the wire and the JSON-RPC error codes. The lines stay counted (no `--exclude`) because an exclusion would hide composition drift; the ~11 uncovered lines are accepted like exception #2. | The bin stays a thin `main`; logic found in it is a review finding and moves to the library or core. | When subprocess profiles are merged reliably on all CI hosts, or the coverage attribute stabilizes. |

The exclusion is deliberately **crate-scoped, not line-scoped**: it can only
shrink, never hide, logic that wrongly migrated into the shell crate.

## 4. Test taxonomy

| Layer | Location | Purpose |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` in the same file | one behavior per test, colocated, fastest feedback |
| Integration | `crates/*/tests/*.rs` | cross-module/cross-crate contracts |
| Property | `proptest` in unit/integration modules | parser/round-trip invariants (decode∘encode∘decode, serde round-trips) |
| Golden file | `fixtures/*.mdb`/`*.accdb` (Phase 2) | byte-level compatibility against the pinned reference databases — provenance and creators per `fixtures/manifest` and docs/FIXTURE-SOURCES.md (jackcess-generated, mdb-reader test data and self-created Access fixtures) |
| Fixture pin integrity | `fixtures/manifest` + CI `fixtures` job | committed sha256 per fixture (`repo@commit` provenance, docs/FIXTURE-SOURCES.md); a drifted or hand-edited fixture fails the build |
| Oracle | `cfg(test)` libmdb backend (Phase 2) | strict schema/row cross-check of our decoder against libmdb (ubuntu + macOS CI, re-admitted 2026-11-07 per L25); the test is `#[ignore]` by default — the CI job runs `-- --ignored` where `DATACRAFT_ORACLE_MDB` is set (L31), so a local skip reads as `ignored` |
| Port contract & capability invariants | `datacraft-core/tests/port_contract.rs` (M21), `availability.rs`, `driver_caps.rs`, `journal_caps.rs`, `crates/datacraft-{jet,sqlite}/tests/*_capabilities.rs` (phase 30+) | cross-driver seam contracts and the declared-capability/affordance vocabulary (ADR-7) |
| Headless E2E | `crates/datacraft-ui/tests/server_e2e*.rs` (Phase 9+) | the full stack over the JSON-RPC wire against real fixture files — assert `UiStateSnapshot` JSON, never pixels |
| Subprocess E2E | `crates/datacraft-cli/tests/cli_process.rs` (T4), `crates/datacraft-mcp/tests/mcp_process.rs` (T7) | the real binaries over real stdio: exit-code matrix, stderr routing, driver inventory |
| Headless UI | `crates/datacraft-ui/tests/ui_kittest*.rs` (T5; families per step 38–42) | real egui widgets driven by AccessKit labels/roles; committed `insta` snapshots and semantic-tree dumps are the layout gate — DPI/font-independent, never pixel compares |
| Layout rects | `crates/datacraft-ui/tests/ui_layout.rs` over the `datacraft_ui::layout_probe` seam (step 44.1, H52/T10) | paint rects recorded by the widgets and asserted as geometry contracts (group widths, in-window bounds, panel non-overlap at two window sizes) — a semantic tree is blind to pixel geometry, which let H52 hide a whole ribbon group behind green tree pins |
| Import contract | `crates/datacraft-import/tests/{contract,cli}.rs` over `fixtures/programmability.accdb` and `fixtures/formDesignsV2016.accdb` (Phase 78, M88) | exact expected-sidecar comparisons, the unsupported/ignored report, strict mode, the merge semantics and the database byte-identity; the generated fixture pins the six control kinds and the omitted-geometry limitation |

**Fixture immutability rule (T3, 2026-11-01):** fixtures are immutable
evidence — never open them in Microsoft Access or any other writer-capable
Jet/ACE application (opening alone rewrites system pages and appends
pages). Manual byte experiments run on a copy; the provenance case study is
docs/FIXTURE-SOURCES.md §4. The two committed `testV2000` variants carry
this history: `testV2000.mdb` is the pristine jackcess pin (`repo@commit`),
`testV2000-access-touched.mdb` documents what real Access does on open.

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
