# PROGRESS_ARCHIVE.p86.md — Shard (Phase 86, append-only)

> Created 2026-11-15T20:36 — home for the detail block of Phase 86 on completion.
> Contains: Phase 86 — Datasheet selection, clipboard and value viewer. Header is write-once — never edited.

---

### Phase 86 — Datasheet selection, clipboard and value viewer

**Scope (Steps):**

- **86.1 Core selection model: anchor rectangle, column selection, Ctrl+A, documented survival rule across scroll/sort/filter (M101)**
- **86.2 Clipboard TSV copy/paste with arity and size validation before any cell changes (M101)**
- **86.3 One-value fill as a single undo step and Delete-to-NULL with the driver NULL boundary (M101)**
- **86.4 Cell value viewer: read-only text/JSON/bounded hex, Shift+F2 and context menu, multiline editor through the 85 buffer (M102)**
- **86.5 Documentation and phase gate (M101, M102)**
**Verification:** Verification: docs_validate clean; the phase's code commits carry the green CI run 43752305677 on main (the earlier 86.3-run failure was the unused_mut the 86.4 push already fixed — its retry covers every phase-86 code commit). Documentation per the phase gate: FEATURES.md's datasheet paragraph names the selection model, the validation-first clipboard, the one-command fill/clear-to-NULL and the value viewer; CHANGELOG's Unreleased gains the coarse Added entry; the README stays untouched (phase-85 precedent — no new format path, client or platform).
