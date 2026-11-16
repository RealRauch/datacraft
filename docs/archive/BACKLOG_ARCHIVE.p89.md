# BACKLOG_ARCHIVE.p89.md — Shard (Phase 89, append-only)

> Created 2026-11-16T21:59 — home for item blocks archived in the context of Phase 89 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 89. Header is write-once — never edited.

---

### [x] M100 — Tool windows as task panes — decision and scope — 🟡
- **Location:** `crates/datacraft-ui/src/tool_window.rs` plus the floating windows it centres (SQL, Expression, Relationships, VBA analysis, Notices).
- **Problem:** Tool windows are free-floating `egui::Window`s with no remembered placement. Access and modern database clients use docked task panes; craft-libs' `panes`/`docking` show the mature model (side/bottom/floating panes, title bar, resize, remembered geometry, transactional dock moves). A migration is a UX decision with a potentially large dependency, not a quick change.
- **Approach:** Write a decision record in `docs/ROADMAP.md` with three options: keep floating windows, build a minimal docked task-pane host in the UI (no new dependency), or adopt craft-ui's panes (git-pinned, unpublished). Compare effort, geometry persistence, multi-window behaviour, testability and the impact on the desktop shell's identity.
- **Acceptance:** A written decision with a go/no-go and, if yes, a scoped follow-up item with acceptance; no implementation before the decision.
- **Steps:** none yet.
- **Done:** done in `5426d79` — ROADMAP §21 decision record: own minimal dock host, no new dependency; implementation follow-up M103 filed (pilot: SQL window)

## ✅ Done Index

> One-liner per completed item (details above).

- M100 — Tool windows as task panes — decision and scope — done (done in `5426d79` — ROADMAP §21 decision record: own minimal dock host, no new dependency; implementation follow-up M103 filed (pilot: SQL window))
