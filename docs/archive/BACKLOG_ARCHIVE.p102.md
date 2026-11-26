# BACKLOG_ARCHIVE.p102.md — Shard (Phase 102, append-only)

> Created 2026-11-26T23:50 — home for item blocks archived in the context of Phase 102 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 102. Header is write-once — never edited.

---

### [x] F18 — Inline fix: moved/resized doc comments attached to the wrong event variants — 🟢
- **Location:** `crates/datacraft-core/src/event.rs` (`ControlContentSet`, `ControlRectSet`, `ReportControlContentSet`, `ReportControlRectSet`).
- **Problem:** The doc line "A control was moved/resized." sat above `ControlContentSet` (and its report pendant above `ReportControlContentSet`), while the rect events carried no doc — rustdoc attributed the movement description to the content command.
- **Fix:** Moved each doc line to its rect event while adding `ControlPropertiesSet` in step 102.1 (commit 60f4e48); no behavior change.
- **Acceptance:** `RUSTDOCFLAGS="-D warnings" cargo doc` stays green; the rect events carry the movement doc.
- **Done:** fixed inline in 60f4e48

---

### [x] M133 — Designer parity: property sheet, resize, z-order, zoom, drag-drop binding (phase 102, roadmap §16 stage 2) — 🟡
- **Location:** `crates/datacraft-core/src/form.rs`/`forms.rs` (control properties, geometry), `crates/datacraft-ui/src/form_design.rs`/`report_design.rs`, sidecar schema.
- **Problem:** The designers are move-only: no control property sheet, no resize handles, no z-order/tab order, a hard-coded preview zoom and click-to-add binding — roadmaps §16 stage 2's parity gap.
- **Approach:** Persist a control property set in the sidecar (name, geometry, style-relevant properties) with core-owned validation and one command per edit; resize handles and z-order/tab-order operations in core geometry; preview zoom as a per-surface setting; drag-and-drop field binding from the field list.
- **Acceptance:** Every placed control exposes its editable properties in both designers; resize and z-order changes survive save/reload; preview zoom persists; dragging a field from the list binds it; invalid geometry/properties fail precisely and leave the design unchanged; kittest pins cover each affordance.
- **Done:** delivered phase 102 (test-first core contracts e504a8a/6330a4f/2508cd0/1232094/b48f70d; implementations 60f4e48/029c13c/461f904/205c8b8)

## ✅ Done Index

> One-liner per completed item (details above).

- F18 — Inline fix: moved/resized doc comments attached to the wrong event variants — done (fixed inline in 60f4e48)
- M133 — Designer parity: property sheet, resize, z-order, zoom, drag-drop binding (phase 102, roadmap §16 stage 2) — done (delivered phase 102 (test-first core contracts e504a8a/6330a4f/2508cd0/1232094/b48f70d; implementations 60f4e48/029c13c/461f904/205c8b8))
