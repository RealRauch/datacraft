# BACKLOG_ARCHIVE.p88.md — Shard (Phase 88, append-only)

> Created 2026-11-16T16:36 — home for item blocks archived in the context of Phase 88 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 88. Header is write-once — never edited.

---

### [x] M98 — Ribbon has no group overflow at narrow widths — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (group row) and the spec in `crates/datacraft-core/src/ribbon.rs`; H52 sized the groups to content and T10 pins them in-window at 1280x800 and 960x600.
- **Problem:** There is no overflow rule below those widths: groups keep their natural widths, so with a narrower window (or a longer localized label) trailing groups leave the window and their actions become unreachable. Access collapses groups into dropdown buttons; craft-libs' `layout::fit_toolbar` documents the policy (keep whole groups, reserve the overflow control's width, never let a command disappear silently).
- **Approach:** Keep the layout math in core (the H52 precedent): compute visible groups plus overflow indices from natural widths, available width, spacing and the overflow control's width. The UI paints one overflow control whose menu dispatches the same `command_for` actions, so disabled states and tooltips stay identical. Decide per group whether to collapse or to list the actions in the overflow menu.
- **Acceptance:** At 800x600 every ribbon action stays reachable (visible or in the overflow list); layout_probe/kittest pins the group widths and the overflow boundary at 800x600, 960x600 and 1280x800 in both languages; no command disappears without a visible entry point.
- **Steps:** none yet.
- **Done:** done in `93fc293` (core fit_groups math, RED `67b2c8b`) + `576eefd` (UI overflow control, RED `4a37401`) — step 88.1: the core math answers a GroupFit plan (whole groups, control width reserved first, total for degenerate rows); the UI paints one localized overflow control whose menu stacks the hidden groups' own action buttons, so dispatch, disabled states and tooltips are identical by construction; pins cover the boundary at 1280x800/960x600/800x600 in both languages, menu reachability at narrow widths and menu dispatch equalling the row command

## ✅ Done Index

> One-liner per completed item (details above).

- M98 — Ribbon has no group overflow at narrow widths — done (done in `93fc293` (core fit_groups math, RED `67b2c8b`) + `576eefd` (UI overflow control, RED `4a37401`) — step 88.1: the core math answers a GroupFit plan (whole groups, control width reserved first, total for degenerate rows); the UI paints one localized overflow control whose menu stacks the hidden groups' own action buttons, so dispatch, disabled states and tooltips are identical by construction; pins cover the boundary at 1280x800/960x600/800x600 in both languages, menu reachability at narrow widths and menu dispatch equalling the row command)
