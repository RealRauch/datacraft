# PLAYBOOK.md — STEPWELL working method (DataCraft adaptation)

> **Project-specific adaptation** of the canonical method file
> (`stepwell-tool/docs/PLAYBOOK.md`, 09/2026), reduced to the rules that apply
> to DataCraft. Upstream method changes are adopted here only where compatible
> with this fork — the upstream "simultaneous in all copies" sync rule is
> therefore dissolved for this copy; the drift is intentional and documented
> here. File names (`BACKLOG.md`, `PROGRESS.md`, `docs/archive/*`) stay
> canonical. Applies to humans and AI agents alike.

## 0. Planning before implementation (workflow — binding)

The order is always: **plan → package → implement.**

1. **Plan:** before any implementation, the steps are planned first (goal, scope, order, risks) — no implementation "off the cuff".
2. **Package:** the plan is broken down in `PROGRESS.md` into concrete assignments with **small steps** (numbered, with status and a short scope per step).
3. **Backlog root (binding):** every step of a phase is derived from at least one open item in `BACKLOG.md` (the item → step reference is documented per item) — the chain finding → item → step stays gapless; planning never arises silently in chat.
4. **Release gate (binding):** the transition from planning and packaging to implementation happens **only after explicit release by the human** — agents package in advance but implement only after release.
5. **Content gates (risk matrix, binding):** the release applies over time (before phase start) **and** by content per file class: **Low** — Rust source/test edits within the step scope → autonomous; **Medium** — `Cargo.toml`/dependency changes, build scripts, new fixture databases → pause, release per occurrence; **High** — CI workflows (`.github/workflows/*`), deleting existing tests, license files, changes to the storage decision (bit-for-bit behavior, clean-room policy) → explicit human gate. Every case is decidable in one sentence: which file class, which level.
6. **Inline-fix lane (the only exception to the step boundary, binding):** a bug discovered during a released, running step may be fixed immediately if it (a) lies within the code scope of the step, (b) is small (≤ ~10 lines; no API/schema/design decision, no new dependency) and (c) touches only the "Low" file class. **Mandatory afterwards:** retro item (series `F`) in `BACKLOG.md` and immediate archival with the commit hash.
7. **Implement:** only then is implementation started — step by step following the rules below (test-first, verification, commit).
8. **Session entry (binding):** every session starts in this order: (1) read this file and `LESSONS.md`; (2) `PROGRESS.md` — identify the next open step of the running phase and mark it 🔄; (3) scan `BACKLOG.md` for open items and blockers. Only afterwards follow write operations to code or docs.
9. **Kickoff fast path (binding):** if the file hashes delivered by `docs_status`/Read-Tools/resources match the last known state, reading the current step scope suffices — this file and `LESSONS.md` need no full re-read. **Rule of thumb:** read independently in parallel, never sequentially.

## 1. Sequential approach

- NEVER work arbitrarily. Work step by step in the defined order.
- Each step builds on the previous one. Start a new step only when the previous one is **verified** complete.
- After the foundation phases: continue working from `BACKLOG.md` by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), same rules (test-first, verification, commit per item/group).

## 2. Documentation duty (living documents — binding)

`README.md`, `PROGRESS.md` and `BACKLOG.md` are **living documents and must always be up to date:**

- **After every step:** maintain the status in `PROGRESS.md` (🔄 before starting, ✅ after verified completion — detail list AND progress table).
- **After every behaviour/feature/environment change:** update `README.md` (features, commands, build).
- **For every found bug/risk/checklist gap:** an entry in `BACKLOG.md` (with location + acceptance criterion).
- **Assignment:** committed phase sequence → `PROGRESS.md` ("Active Phases"); everything open **outside** this sequence (ideas, requests, risks, findings) → item in `BACKLOG.md`.
- **No code without a step in `PROGRESS.md`, no known problem without an entry in `BACKLOG.md`.**

### Language (binding)

All **project artifacts are English**: code, comments, documentation (README/PLAYBOOK/LESSONS/AGENTS), commit messages, test names, and Stepwell structure AND content. Conversation with the human may follow the human's language; nothing written into the repository is German.

## 3. Growth limit (archive pattern)

`BACKLOG.md` and `PROGRESS.md` contain only **open** content:

- **Completed backlog item:** move the full block **unchanged** into `docs/archive/BACKLOG_ARCHIVE.md`; `BACKLOG.md` keeps a one-liner in the Done Index (canonical format: `- <ID> — <short text> — done in <sha>`).
- **Completed phase:** move the detail block **unchanged** into `docs/archive/PROGRESS_ARCHIVE.md`; the **progress table stays complete** in `PROGRESS.md` (permanent index).
- Archives are **append-only history** — their content is never edited afterwards.

### Item-ID nomenclature (binding)

- **Series letter:** `K`/`H`/`M`/`L` = priority series (🔴/🟠/🟡/🟢); other letters = thematic series (e.g. `T` = test gaps, `F` = inline-fix retrospectives).
- **🔵 = test gaps:** thematic series IDs (e.g. `T5`), assigned explicitly — no auto-number from a priority series.
- **Numbers:** consecutive within the series, ascending, **never reused** — not even after archiving. IDs are project-wide unique (case-sensitive, exact comparison).
- **Separate namespaces:** item IDs and step numbers (`<phase>.<no.>`) have nothing to do with each other.

### Timestamps (binding)

Date/time entries in the four doc files always use **ISO 8601** `YYYY-MM-DDTHH:mm` (local time) — affects `*(done …)*`/`*(completed …)*` markers and `As of:` header lines. No legacy formats are created.

## 4. Commit discipline

- Commit after every completed step, mandatory after every fully completed phase.
- Before the commit: inspect `git status`, `git diff`, `git log --oneline -10`.
- Stage only intended files. No secrets, no `.env` files.
- Consistent, short commit messages (imperative, English, lowercase, e.g. `feat: parse jet page headers`).
- Test-first chain auditable in the log: new tests first in their own `test(scope): …` commit (suite RED; failure evidence in the commit message or step note), then `feat(scope): …` with the implementation.
- If a commit fails (hooks): fix the error and create a new commit — do not amend.

## 5. Verification before completion

- Never mark a step as "done" without passed verification. The canonical verification command set:

  ```
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo llvm-cov --workspace --lcov --output-path lcov.info --fail-under-lines 95
  ```

  plus the file-size gate (no `.rs` file ≥ 500 lines; enforced by CI). All green — or the step is not done.
- **Hard quality gates** (coverage ≥ 95 %, file size < 500 lines, clippy `-D warnings`) are enforced by CI and are **absolute** — they replace any smell budget. Soft smells (feature envy, god concept, naming) are findings → items in `BACKLOG.md`, never gates.
- **No technical debt (binding):** deficiencies never live in the code. No `TODO`/`FIXME`/`HACK`/`XXX` markers anywhere in tracked files (mechanically scanned by the CI hygiene job); "temporary" code does not exist — code is final or not committed. Every known deficiency becomes a `BACKLOG.md` item the moment it is known. Deferred features are documented scope decisions (ARCHITECTURE/BACKLOG), not debt: debt means knowingly leaving the codebase inconsistent with its own rules.
- **Every coverage exception** must be listed with justification in `docs/TESTING.md` — the exception list is part of the quality gate.

### Intentionally accepted findings (inline marker)

A deliberate, documented deviation from a consistency rule is marked **on the line the finding points at** with `*(accepted: CODE[, CODE])*` (codes compared case-insensitively), e.g.

```
- R1-A — legacy index entry — done in `d4e5f6a` *(accepted: ID_CONVENTION)*
```

Marked findings move into a separate `acknowledged[]` list: they stay **visible** but no longer turn `docs_validate` `ok` to false — a new, unmarked finding still blocks. The append-only archives are exempt (never faulted, never need a marker).

## 6. Test-first (RED → GREEN) — binding

- **Every written code must be covered by tests.** No feature/step code without associated tests.
- Order: **first** the tests (RED — they fail because the function is missing), **then** the implementation until GREEN.
- **RED is evidenced, not claimed:** the new tests land first in their own `test(scope): …` commit with failure evidence, only then the `feat(scope): …` implementation commit (see §4).
- Exceptions only when tests are objectively not sensible (e.g. pure configuration files); the reason is noted in the commit.
- **Verification as an executable plan:** the verification line of a step names at least one **runnable command** and the **expected result** (e.g. the §5 command set — all green).

## 7. Status legend

⬜ open · 🔄 in progress · ✅ done · ⛔ blocked (with a short reason).

A blocked step is **never** reported as done; the blocker — provided it is a genuine open point — belongs as an item in `BACKLOG.md` so it is not lost.

## 8. Method retrospective (periodic pruning)

Every **5 phases** (this project's declared interval) the acting team reviews `LESSONS.md` and this playbook as a whole. Three guiding questions per rule: (1) Did the rule prevent a bug? (2) Does it cost more than it catches? (3) Can it be consolidated? Every rule ends in exactly one outcome: **stay**, **demote** (binding → recommended) or **drop** (text preserved with a pointer in the archive). There is no obligation to prune — but this is the legitimized path for dropping rules. Changes to this fork are committed here (upstream sync is no longer automatic).
