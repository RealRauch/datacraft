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

---

### [x] L46 — Native menu bar on macOS — decision — 🟢
- **Location:** the shell roots (`crates/datacraft-ui/src/app.rs`, `shell.rs`) and the macOS target gate; no menu-bar module exists.
- **Problem:** macOS apps are expected to provide a menu bar (App/File/Edit/View/Window/Help) with the standard About, Settings, Services, Hide and Quit items, and a Window menu listing open documents. DataCraft ships a ribbon-only shell, so those platform affordances are absent and standard chords such as Cmd+Q are not routed through the app's policies. craft-libs' `craft-menu` documents the role-based layout (items found by role, not title) and a native macOS backend.
- **Approach:** A time-boxed decision: whether to add a macOS-only menu bar that dispatches the existing command vocabulary (no new commands), built either with a small own module or an adopted menu crate. The ribbon stays the primary surface on every platform.
- **Acceptance:** A written decision record; if yes, the menu items dispatch existing commands only, their key equivalents route through the same shortcut model as M99, and Windows/Linux stay visually and behaviourally unchanged.
- **Steps:** none yet.
- **Done:** done in `43aebbd` — ROADMAP §22 decision record: ribbon-only, no native menu bar now; re-evaluation gated on reachable macOS hardware for verification

## ✅ Done Index

> One-liner per completed item (details above).

- M100 — Tool windows as task panes — decision and scope — done (done in `5426d79` — ROADMAP §21 decision record: own minimal dock host, no new dependency; implementation follow-up M103 filed (pilot: SQL window))
- L46 — Native menu bar on macOS — decision — done (done in `43aebbd` — ROADMAP §22 decision record: ribbon-only, no native menu bar now; re-evaluation gated on reachable macOS hardware for verification)
