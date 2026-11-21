# BACKLOG_ARCHIVE.p93.md — Shard (Phase 93, append-only)

> Created 2026-11-21T20:45 — home for item blocks archived in the context of Phase 93 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 93. Header is write-once — never edited.

---

### [x] M125 — Dock rollout: the remaining dockable tool surfaces — 🟡
- **Location:** `crates/datacraft-ui/src/relationships.rs` (`mapper_window` over `ToggleRelationshipMapper`/`relationship_mapper_open`) and `crates/datacraft-ui/src/expression_window.rs` (`expression_window` over `ToggleExpressionWindow`/`expression_window_open`); the host API lives in `crates/datacraft-ui/src/panes.rs` (`PaneSpec`/`pane_panel`, step 92.2).
- **Problem:** M108 shipped the dock host with the SQL window as pilot and deferred the remaining surfaces "per-surface items after the pilot proves the model". The pilot proved the model (dock/float/restart pinned, the no-overlap layout contract green at both window sizes). The two remaining floating exploration surfaces — the Relationships mapper and the Expression window — still open centered with no remembered placement. (The VBA viewer renders inside a document tab and the Import File window is a transient flow — both stay floating by design.)
- **Approach:** migrate each surface to `pane_panel` with its own `PaneSpec` (surface keys `"relationships"`/`"expression"`, probe ids `panes.relationships_panel`/`panes.expression_panel`), the header dock menu plus its context menu, the floating window keeping the Dock menu; a docked panel renders the content without the `egui::Window`; geometry rides the existing ui.json pane persistence (92.2's sane clamps).
- **Acceptance:** each surface docks right/bottom and floats; the workspace never overlaps a docked panel (layout pins at 1280x800/960x600); the geometry survives restart; every string goes through `t()`/`tr()`; no new dependency; the file-size gate stays green (split before 500).
- **Steps:** none yet.
- **Done:** steps 93.1–93.2 — done in `366ca3b` (relationships dock) and `67b9276` (expression dock; carries the pane-header t() fix)

---

### [x] L64 — Object Details gains its Type column (the grid overview) — 🟢
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (the details branch) and `crates/datacraft-ui/src/nav_row.rs` (`paint_row`).
- **Problem:** L48's list/grid overview shipped as the flat Object Details list (icon + name rows, step 92.5); its archive note kept "a grid with metadata columns" as a follow-up. Today the details list carries no metadata beyond the kind icon, so a reader cannot scan what kind each object is without reading the icon.
- **Approach:** in details mode each row renders its name plus the localized kind label as a trailing muted column (the same display vocabulary as the design grid's kind column); rows keep the search filter, the nav sort (Name/Type) and the multi-select semantics; a kittest pins the type column in both languages; the behaviour note rides FEATURES.md.
- **Acceptance:** the details list shows name + localized kind per row; sorting by Name/Type reorders; searching still narrows; the column never touches the sidecar or the database file.
- **Steps:** none yet.
- **Done:** step 93.3 — done in `da1f41d` (labels::object_kind_label + the accessibility value; the German catalog gains the five singular kind labels)

---

### [x] R3 — Method retrospective (PLAYBOOK §8) — phases 89–93 checkpoint — 🟢
- **Scope:** the project's first PLAYBOOK §8 retrospective — `docs/LESSONS.md` (all 32 rules) and `AGENTS.md` against the phases 89–93 window.
- **Verdict: every rule stays** — no demotion, no drop; no PLAYBOOK/LESSONS text changed, so no sync across method copies is required.
- **Rules that caught something in the window (evidence):** the file-size gate forced three splits in step 91.13 (`form_runtime.rs` 534 → 484, `port.rs` → `port/family.rs`, `grid.rs` → the `paint_row_chrome` helper); test-first's existing pins caught step 93.2's header extraction deterministically (the missing `ui.horizontal` row killed the window's interactions); LESSONS #28 held twice — the m99 platform pins failed on macOS CI (the Windows machine cannot see ⌘-forms; fixed in `7d5545d`/`f26b8ac`) and the phase-93 push closed the phase with the green run 44942643622; LESSONS #17 held — every structure op ran through the stepwell tools, two accidental dry-run calls were caught by reading the tool output, no drift.
- **Central finding (LESSONS #26 applied to itself):** the i18n rule (AGENTS §6, "UI strings go through `t()`/`tr()`") is prose-only — the pane-header bypass (step 93.1) survived to acceptance review because the pilot's title is identical in German. → filed as **T12** (mechanical t()-bypass scan in the CI hygiene job).
- **Overhead:** no rule cost more than it caught; AGENTS §3 mirrors PLAYBOOK §0.5 deliberately (AGENTS is the summary, PLAYBOOK wins on conflict) — no consolidation. The web/API-scoped LESSONS section A stayed dormant (no such surface changed in the window).
- **Outcomes:** T12 filed; AGENTS §8 documents the transient Windows LNK1104 rerun hint (`-j 2`); the PROGRESS header declares the interval — every 5 phases, next checkpoint at phase 98.
- **Done:** completed at the phase-93 milestone — doc checkpoint `4270869`; follow-ups: T12 (open), AGENTS §8 note and the PROGRESS retro-interval declaration ride `4270869`

## ✅ Done Index

> One-liner per completed item (details above).

- M125 — Dock rollout: the remaining dockable tool surfaces — done (steps 93.1–93.2 — done in `366ca3b` (relationships dock) and `67b9276` (expression dock; carries the pane-header t() fix))
- L64 — Object Details gains its Type column (the grid overview) — done (step 93.3 — done in `da1f41d` (labels::object_kind_label + the accessibility value; the German catalog gains the five singular kind labels))
- R3 — Method retrospective (PLAYBOOK §8) — phases 89–93 checkpoint — done (completed at the phase-93 milestone — doc checkpoint `4270869`; follow-ups: T12 (open), AGENTS §8 note and the PROGRESS retro-interval declaration ride `4270869`)
