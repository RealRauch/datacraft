# PROGRESS_ARCHIVE.p84.md — Shard (Phase 84, append-only)

> Created 2026-11-15T11:36 — home for the detail block of Phase 84 on completion.
> Contains: Phase 84 — Wider pruning classes. Header is write-once — never edited.

---

### Phase 84 — Wider pruning classes

**Scope (Steps):**

- **84.1 clean-targets.ps1: depth-2 cargo targets (src-tauri) plus node_modules/.venv classes with the stale rule (M107)**
- **84.2 Report pin of the four Tauri targets, apply run as the one-time clean, CONTRIBUTING documentation and phase gate (M107)**
**Verification:** Gate review of phase 84: the widened classes are pinned by the report run (four src-tauri targets + 24 npm + 10 venv entries, all stale, 3 s runtime), the apply run removed 34 dirs and freed +53 GB (279 -> 332 GB on D:), the four Tauri targets verified gone, fresh+small entries (incl. datacraft/target) untouched; M107 archived, CONTRIBUTING documents the widened classes; the scheduled task needed no change (same script path, new policy rides it).
