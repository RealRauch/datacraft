# PLAYBOOK.md — STEPWELL working method (project-independent, reusable)

> Generalized method, extracted from the Stadtpfad project (09/2026).
> The file names (`BACKLOG.md`, `PROGRESS.md`, `docs/archive/*`) are part of the pattern —
> adopt them unchanged when copying into a new project.
> Applies to humans and AI agents alike.
> The method is called **STEPWELL** — like a stepwell: many small, each
> load-bearing steps, layer upon layer.
> Method changes (this file, LESSONS.md) happen **simultaneously in all copies** —
> if a project copy drifts, add a sync entry to its `BACKLOG.md` immediately.

## 0. Planning before implementation (workflow — binding)

The order is always: **plan → package → implement.**

1. **Plan:** before any implementation, the steps are planned first (goal, scope, order, risks) — no implementation "off the cuff".
2. **Package:** the plan is broken down in `PROGRESS.md` into concrete assignments with **small steps** (numbered, with status and a short scope per step).
3. **Backlog root (binding):** every step of a phase is derived from at least one open item in `BACKLOG.md` (the item → step reference is documented per item) — the chain finding → item → step stays gapless; planning never arises silently in chat.
4. **Release gate (binding):** the transition from planning and packaging to implementation happens **only after explicit release by the human** — agents, like humans, package in advance but implement only after release.
5. **Content gates (risk matrix, binding):** the release applies over time (before phase start) **and** by content per file class: **Low** — source/test edits within the step scope → autonomous; **Medium** — dependency manifests, Dockerfiles → pause, release per occurrence; **High** — `.env`, CI workflows, deleting existing tests, schema migrations → explicit human gate. Every case is decidable in one sentence: which file class, which level.
6. **Inline-fix lane (the only exception to the step boundary, binding):** a bug discovered during a released, running step may be fixed immediately if it (a) lies within the code scope of the step, (b) is small (rule of thumb ≤ ~10 lines; no API/schema/design decision, no new dependency) and (c) touches only the "Low" file class. **Mandatory afterwards:** retro item (series `F`) in `BACKLOG.md` and immediate archival with the commit hash — the chain finding → item → Done Index stays gapless. Everything else remains a regular open item.
7. **Implement:** only then is implementation started — step by step following the rules below (test-first, verification, commit).
8. **Session entry (binding):** every session starts in this order: (1) read this file and `LESSONS.md`; (2) `PROGRESS.md` — identify the next open step of the running phase and mark it 🔄; (3) scan `BACKLOG.md` for open items and blockers. Only afterwards follow write operations to code or docs.
9. **Kickoff fast path (binding):** If the file hashes delivered by `docs_status`, the Read-Tools or resources match the last known state, reading the current step scope suffices — this file and `LESSONS.md` do not need to be re-read in full. The fast path applies only where hashes are actually delivered (capability since E2/12.5). **Rule of thumb:** place independent read calls in parallel — never read sequentially what can be read in parallel.

## 1. Sequential approach & parallel workstreams

- NEVER work arbitrarily. Within a workstream (see below), work step by step in the defined order.
- Each step builds on the previous one. Start a new step only when the previous one is **verified** complete.
- After the foundation phases: continue working from `BACKLOG.md` by priority (🔴 → 🟠 → 🟡 → 🟢 → 🔵), same rules (test-first, verification, commit per item/group).

### Parallel phases as workstreams (binding, 11/2026, H5)

A project may run several phases **in parallel** — as **workstreams**: each workstream is
one phase in its **own git worktree + branch**. Agents never share a working tree; foreign
changes reach the own tree only through the own sync step. Parallelism requires git — a
project without git stays single-tree sequential (one running phase at a time).

**Per-step ritual (divergence ≤ 1 step):**

```
pull (trunk → branch) → implement → code commit (agent)
→ doc commit (tool, doc file set only) → pull (trunk → branch)
→ verify (typecheck + suite) → push (branch → trunk)
```

- **Push precondition (binding):** a push is allowed only if the trunk head is already an
  **ancestor of the branch** (the preceding pull brought it in). A stale push is refused
  with "re-pull required" → pull → re-verify → push. What gets pushed was verified against
  the current trunk. Git's `index.lock` and atomic branch moves serialize the residual
  window; races degenerate into visible merge conflicts (clean abort + structured report),
  never silent lost updates. On repeated push failure: pause and re-sync — no livelock.
- **Doc conflicts:** keep both sides; in archives, order blocks by completion timestamps.
- **WIP visibility (binding):** the doc commit happens at step **start** (🔄) as well as at
  completion (✅) — the trunk always shows the true state. Workstream liveness is derived
  from git itself (existing workstream branches + last-commit times), never from a registry
  file or heartbeat.
- **Trunk derivation (zero config):** trunk = the branch checked out in the main worktree.
- **Session entry in a workstream:** pull first, then the §0.8 order — the docs read are
  the freshly synced state.
- **Count guidance:** as many workstreams as useful — each additional one multiplies the
  per-step merge traffic; two or three are typically practical.
- **Lifecycle:** worktree/branch creation and cleanup (after phase completion + push) are
  agent rituals, not tool operations.

## 2. Documentation duty (living documents — binding)

`README.md`, `PROGRESS.md` and `BACKLOG.md` are **living documents and must always be up to date:**

- **After every step:** maintain the status in `PROGRESS.md` (🔄 before starting, ✅ after verified completion — detail list AND progress table).
- **After every behaviour/feature/environment change:** update `README.md` (features, commands, env variables, deploy).
- **For every found bug/risk/checklist gap:** an entry in `BACKLOG.md` (with location + acceptance criterion).
- **Assignment:** the committed phase sequence lives in `PROGRESS.md` ("Active Phases") — also roughly packaged (fine-packaging at phase start). Everything open **outside** this sequence (ideas, feature requests, risks, findings) belongs as an item in `BACKLOG.md` — with location + acceptance criterion. Decision aid: "Does it belong to the committed phase sequence?" → PROGRESS; otherwise → BACKLOG.
- The rule holds: **no code without a step in `PROGRESS.md`, no known problem without an entry in `BACKLOG.md`.**

### Language (binding, 09/2026)

The structural language of the method is **English primary**: section headings, field labels (Location, Acceptance, Verification, Scope), the Done Index (was: Erledigt-Index), status words, tool descriptions, CLI help, warning messages, templates, PLAYBOOK/LESSONS/AGENTS/README/SKILL. **Content texts** (item bodies, notes, user prose) stay deliberately language-free — everyone writes them in their language. Rationale: structure/API code and prose stay congruent; non-German speakers can read the method. **Transition tolerance:** legacy DE-labels in open files are read **silently** by the parser (union-matching reads DE+EN; decision I1/14.6) — **no** `STRUCT_LOCALE` warning in `docs_validate`. The tolerance is part of the parser semantics, not an observation duty: anyone who touches an item anyway flips its labels to EN at the natural edit. Archives are never faulted. Migration: open files at the next natural edit (Phase 14, Step 14.2 in `stepwell-tool`); archives never touched; new projects start directly with EN templates. Method-changes to this language convention apply synchronously in both PLAYBOOK/LESSONS copies (textually identical, SHA256-evidence per copy).

## 3. Growth limit (archive pattern)

`BACKLOG.md` and `PROGRESS.md` contain only **open** content:

- **Completed backlog item:** move the full block **unchanged** into `docs/archive/BACKLOG_ARCHIVE.md`; `BACKLOG.md` keeps a one-liner in the Done Index (with commit hash).
- **Completed phase:** move the detail block **unchanged** into `docs/archive/PROGRESS_ARCHIVE.md`; the **progress table stays complete** in `PROGRESS.md` (permanent index).
- Archives are **append-only history** — their content is never edited afterwards.

### Archive sharding (binding, 11/2026)

Large projects split the archive into **shards per phase** — the canonical files
stay the mandatory home for the legacy stock and for appends **without a running
phase**:

- **Shard naming:** `docs/archive/BACKLOG_ARCHIVE.p<n>.md` and `docs/archive/PROGRESS_ARCHIVE.p<n>.md` — named by **phase number**, not sequence: the permanent progress table is the discovery index (phase → file visible in the name); no manifest file (that would be a second truth). The shard set is order-free; chronology lives in the completion timestamps inside the blocks.
- **Rotation is a tool op** (`archive_item`, `backlog_remove`, `progress_update`): item blocks archived **while phase n runs** append directly to `BACKLOG_ARCHIVE.p<n>.md`; on phase completion the detail block appends to `PROGRESS_ARCHIVE.p<n>.md`. Each shard gets a **write-once header** (phase, creation stamp, contains line) — created by the tool, never edited afterwards.
- **Coupled Done Index:** an index line lives **where its block lives** — BACKLOG's Done Index for canonical blocks, each shard's own Done-Index section for shard blocks. There is no single index of "everything completed" — the read path is the tool (concatenated view) or distributed grep.
- **Strict locality:** an index line in a different file than its block is the `INDEX_LOCATION` finding — move the line to the file of its block.
- **Legacy rule:** existing archives are never split retroactively (append-only, no bulk migration) — the canonical file keeps the pre-sharding stock. A clean cut is possible per project via the sanctioned rename op (`*_ARCHIVE.md` → `*_ARCHIVE.legacy.md`); it is optional and not part of the method.

### Item-ID nomenclature (binding)

Item IDs follow the pattern `<series letter><number>` (e.g. `H1`, `R4`, `U21`):

- **Series letter:** `K`/`H`/`M`/`L` = priority series (🔴/🟠/🟡/🟢); other letters = thematic series (e.g. `T` = test gaps, `R`/`U` = review/topic series).
- **🔵 = test gaps:** the 🔵 priority marks test gaps as an item class; such items carry thematic series IDs (e.g. `T5`) and receive **no** auto-number from a priority series — the ID is assigned explicitly.
- **Number:** within the series consecutive, ascending, **never reused** — not even after archiving (the append-only history tolerates no ID collisions).
- **Uniqueness:** IDs are project-wide unique across open items, Done Index and archives — case-sensitive, exact comparison.
- **Separate namespaces:** item IDs (`BACKLOG.md`) and step numbers in the progress table (`<phase>.<no.>`, e.g. `2.1`) have nothing to do with each other.

### Timestamps (binding)

Date/time entries in the four doc files are always created in the **ISO 8601** format
`YYYY-MM-DDTHH:mm` (local time, e.g. `2026-09-07T15:23`) — this affects the
`*(done …)*` and `*(completed …)*` markers as well as `As of:` header lines. The
legacy stock (`YYMMDD/HHMM`, e.g. `260907/1523`, and the 4-digit `MM/YYYY`) remains
valid (tolerance, read silently) but is no longer created. `docs_validate` reports
**both** legacy forms as `DATE_LEGACY` — only in open files, never in the
append-only archive.

### Migration of existing projects (binding)

- Archives are **never** edited afterwards (append-only) — legacy formats remain permanently valid there.
- Open files (`BACKLOG.md`, `PROGRESS.md`): conversion to ID nomenclature and timestamp format happens at the next natural edit — no bulk rebuild, no special action.
- Convention warnings (`ID_CONVENTION`, `DATE_LEGACY`) affect **only open files**, never archives.
- New projects create **all four files** (including empty archives) at project start.

### Done Index formats (binding)

- **Canonical** is the bullet one-liner per completed item: `- <ID> — <short text> — done in <sha> …` — all write paths (including tools) produce exactly this format.
- **Tolerated when reading:** a Done Index as a table (`| Series | Item (short) | Commit/Phase |`) is recognised and parsed (column `Item` → ID + short text, column `Commit/Phase` → commit hash); it is not created anew — existing tables migrate to one-liners at the next natural edit.
- **Compact lines are drift:** range lines (`L1–L11 …`) and collection lines (`R1 … + R1-… …`) do not count as a per-item index entry — validation reports every `[x]` archive block without its own one-liner as a finding.

### Adoption of existing projects (binding)

Introducing STEPWELL into a project **without** the four files (grey-/brownfield) follows this model — greenfield projects start directly with all four files:

1. **Adoption = snapshot:** the four files are created at the adoption commit with the AS-IS state: `BACKLOG.md` with only the **known** open points, Done Index and archives **empty**, `PROGRESS.md` with exactly one line `0.1 STEPWELL adoption (baseline <sha>)` ✅ — no retroactive history, ever.
2. **Budgeted inventory:** a time-boxed inventory step (head knowledge, TODO/FIXME scan, issue import) fills `BACKLOG.md` with the most important items. Afterwards, backlog knowledge only arises in work: finding → item.
3. **Verification levels (declared in the `PROGRESS.md` header line):** **Level 0** — no automated test (verification as a test protocol in the step note; new core logic brings its own test) · **Level 1** — characterization/golden-master tests (they may be green — observation before specification) · **Level 2** — full RED→GREEN. A level change is a commit.
4. **Strangler principle:** the STEPWELL standards apply to new work and touched zones — no remediation phase, no bulk migration of the existing stock.

## 4. Commit discipline

- Commit after every completed step, mandatory after every fully completed phase.
- Before the commit: inspect `git status`, `git diff`, `git log --oneline -10`.
- Stage only intended files. No secrets, no `.env` files.
- Consistent, short commit messages in the repo style (imperative, English, lowercase, e.g. `feat: add env config`).
- Test-first chain auditable in the log: new tests first in their own `test(scope): …` commit (the suite is RED; the failure evidence is in the commit message or the step note), then `feat(scope): …` with the implementation.
- If a commit fails (hooks): fix the error and create a new commit — do not amend.
- **Doc commits by the tool (workstream mode, binding):** in workstream mode the tool commits
  only the doc file set (canonical four + existing archive shards — deterministic staging, no secrets risk, nothing else is staged);
  code commits remain the agent's duty. A failed doc commit (e.g. hooks) leaves the edits
  applied with a structured message — never a rollback; the agent resolves and commits itself.

## 5. Verification before completion

- Type check (strict), unit/integration suite and E2E suite must run **without errors** (project-specific commands: see `AGENTS.md`).
- **Zero errors, zero warnings (binding, 09/2026, M105):** the test suites must run **without stderr warnings** — `act(...)`-warnings, deprecations, unhandled rejections and similar count as findings, not as noise. Every warning trains the eye to overlook stderr, and exactly there land the real failures later; each warning is therefore fixed at its cause (await updates to completion, `fireEvent`/`userEvent` instead of raw DOM events, controlled fake-timer teardown) — never suppressed globally.
- Never mark a step as "done" without passed verification.
- **Workstream verification order (binding, 11/2026, H5):** in workstream mode verification
  runs **after the pull, before the push** — the suite proves the merged state (own step +
  trunk's current state), and only a verified merged state moves the trunk.
- **CI is part of verification (F7, 09/2026):** the local machine is one OS/scheduler — timing-dependent behaviour (concurrency, locks) and pipeline-only steps (workflow plumbing, `docker run`) are only exercised where CI actually runs. Projects with a CI pipeline therefore push **at every phase boundary at the latest** (push interval ≤ phase) and count the phase's CI run as part of its verification: a phase is not "done" until its commits have a green CI run.
- **Optional — smell budget (quality gate):** for those who declare it in the `PROGRESS.md` header line, the following applies: (1) **soft smells** (feature envy, god concept, naming) are findings → items in `BACKLOG.md`, never gates. (2) **hard smells** (file/function length, complexity, duplication, lint rules) form a **delta budget**: a step must not increase the smell load of the files it touches; absolute thresholds only in greenfield from day 1, thresholds are only lowered and only updated at the natural occasion. (3) A delta report is release context, never a blocker — the gate stays binary (budget kept yes/no, checkable locally or in CI). Those who declare nothing have no budget.

### Intentionally accepted findings (inline marker)

A deliberate, documented deviation from a consistency rule is marked **on the line the
finding points at** with `*(accepted: CODE[, CODE])*` (codes compared case-insensitively),
e.g.

```
- MILESTONE-Alpha — legacy index entry — done in `d4e5f6a` *(accepted: ID_CONVENTION)*
```

Such findings move into a separate `acknowledged[]` list: they stay **visible** (the count
is reported) but no longer turn the validation result `ok` to false — a new, unmarked
finding still blocks. There is deliberately **no config file and no extra tool**: the
marker lives inside the already-tolerated optional `*(…)*` suffix, so titles and IDs stay
clean, and findings without a source line (or without the marker) stay blocking.

The **append-only archives are exempt**: their content is never edited afterwards, so legacy
formats and conventions there are never faulted and never need a marker.

## 6. Test-first (RED → GREEN) — binding

- **Every written code must be covered by tests.** No feature/step code without associated tests.
- Order: **first** write the tests (they initially fail = RED because the function is still missing), **then** the implementation until the tests are green (GREEN).
- **RED is evidenced, not claimed:** the RED phase is part of the evidence — the new test first lands in its own `test(scope): …` commit with failure evidence (commit message or step note), only then the `feat(scope): …` implementation commit (see §4).
- This leaves a reproducible test suite at the end of every step that proves the implementation.
- Exceptions only when tests are objectively not sensible (e.g. pure configuration/structure files); the reason is noted in the commit.
- New tests belong in the central test directory and are picked up by the standard suite.
- **Verification as an executable plan (convention):** the verification line/list of a step (step note, commit message or the `**Verification:**` line on the archive block) names at least **one runnable command** and the **expected result** (e.g. `npm run typecheck && npm run test` — both green). Deliberately without a validate warning — pure convention, no gate.

## 7. Status legend

⬜ open · 🔄 in progress · ✅ done · ⛔ blocked (with a short reason).

A blocked step is **never** reported as done; the blocker — provided it is a genuine open point — belongs as an item in `BACKLOG.md` so it is not lost.

## 8. Method retrospective (periodic pruning — binding)

The method itself is subject to step-wise care, not to accumulation. Every **N phases**
(recommended: 5; a project may declare a different interval in its `PROGRESS.md` header
line) the acting team reviews `LESSONS.md` and the architecture decisions (`AGENTS.md`)
as a whole. Three guiding questions per rule:

1. **Did the rule prevent a bug in the last N phases?** (evidence: an item, a finding or
   a reverted change it actually caught)
2. **Does it produce more overhead than benefit?** (cost in time/context versus the
   failures it catches)
3. **Can it be consolidated with another rule?** (two rules with the same trigger)

Every rule ends in exactly one outcome:

- **stay** — the rule stays as it is, binding.
- **demote** — binding → recommended, documented in place.
- **drop** — the rule is removed; its text is preserved with a pointer in the archive
  (append-only) so the reasoning stays traceable.

There is **no obligation to prune** — a retrospective may well conclude "everything
stays". But it provides a **legitimized path for dropping rules**, so that the method
does not grow monotonically. The retrospective itself is documented (a short note at the
phase milestone or its own step); rule changes apply synchronously in all
PLAYBOOK/LESSONS copies.
