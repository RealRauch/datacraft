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

## ✅ Done Index

> One-liner per completed item (details above).

- F18 — Inline fix: moved/resized doc comments attached to the wrong event variants — done (fixed inline in 60f4e48)
