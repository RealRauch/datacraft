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

## ✅ Done Index

> One-liner per completed item (details above).

- M125 — Dock rollout: the remaining dockable tool surfaces — done (steps 93.1–93.2 — done in `366ca3b` (relationships dock) and `67b9276` (expression dock; carries the pane-header t() fix))
- L64 — Object Details gains its Type column (the grid overview) — done (step 93.3 — done in `da1f41d` (labels::object_kind_label + the accessibility value; the German catalog gains the five singular kind labels))
