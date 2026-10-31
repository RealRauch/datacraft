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
- **Every exception must be registered below.** Unregistered exclusions are a
  High-class review finding (quality gate manipulation).

## 3. Coverage exception registry

| # | Scope | Mechanism | Justification | Rules | Revisit |
|---|-------|-----------|---------------|-------|---------|
| 1 | crate `datacraft-ui` | `--exclude datacraft-ui` on the llvm-cov invocation (CI + local) | The crate is eframe/egui rendering glue: the event-loop entry point (`main`) and `eframe::App` paint code are not exercisable headlessly. `#[coverage(off)]` is **unstable** on the pinned stable toolchain (rust 1.98, rust-lang/rust#84605), so the attribute cannot be used. | Rendering glue only — **all logic lives in `datacraft-core`** (covered). Logic found in `ui` (layout math, state, commands) is a review finding and moves to core. | When the coverage attribute stabilizes, or if eframe enables headless `App` execution. |

The exclusion is deliberately **crate-scoped, not line-scoped**: it can only
shrink, never hide, logic that wrongly migrated into the shell crate.

## 4. Test taxonomy

| Layer | Location | Purpose |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` in the same file | one behavior per test, colocated, fastest feedback |
| Integration | `crates/*/tests/*.rs` | cross-module/cross-crate contracts |
| Property | `proptest` in unit/integration modules | parser/round-trip invariants (decode∘encode∘decode, serde round-trips) |
| Golden file | `fixtures/*.mdb`/`*.accdb` (Phase 2) | byte-level compatibility against reference databases created with MDB Tools tooling only |
| Oracle | `cfg(test)` libmdb backend (Phase 2) | strict schema/row cross-check of our decoder against libmdb |
| Headless E2E | `datacraft-core` command bus (Phase 9) | assert `UiStateSnapshot` JSON — never pixels |

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
