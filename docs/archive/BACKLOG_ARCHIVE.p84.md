# BACKLOG_ARCHIVE.p84.md — Shard (Phase 84, append-only)

> Created 2026-11-15T11:36 — home for item blocks archived in the context of Phase 84 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 84. Header is write-once — never edited.

---

### [x] M107 — Pruning policy for depth-2 cargo targets and JS/Python artifacts — 🟡 — 🟡
- **Location:** `scripts/clean-targets.ps1` and the machine's build-artifact homes beyond this workspace: the measurement of 2026-11-15 found ~62 GB across the development root, of which ~47 GB sit in four nested Rust targets that the phase-83 policy cannot see (they live at `<project>\src-tauri\target`, one level deeper than the scanned `<project>\target`) plus the `node_modules`/`.venv` trees that are not cargo artifacts at all.
- **Problem:** the keep-only-the-last-build policy (M106) covers only depth-1 cargo targets; the biggest artifact blocks on the machine escape it, and the human released both the one-time clean and the policy extension (session 2026-11-15).
- **Approach:** extend the detection walk to two levels with class rules — `target` counts when its parent carries a `Cargo.toml` (this admits `src-tauri/target`), `node_modules` counts when its parent carries a `package.json`, and `.venv`/`venv` count unconditionally; the stale rule (older than `-MaxAgeDays`) then applies to every class, the oversized rule (`-MaxSizeGB`) as before; fresh and small stays untouched, so active projects never lose a usable build. The apply run doubles as the one-time clean of the four Tauri targets.
- **Acceptance:** the report run names the four `src-tauri\target` dirs with their verdicts; the apply run frees ~47 GB on D:; fresh `node_modules` trees survive untouched; CONTRIBUTING documents the widened classes; the scheduled task needs no change (same script path).
- **Done:** done (phase 84) — clean-targets.ps1 probes two levels per project (admitting Tauri's src-tauri\target) and covers cargo/node_modules/.venv with the stale rule; the root-caused -Depth-1 miss (depth 2 from the scan root) was replaced by targeted Test-Path probes, making the report run take 3 s instead of a full-recursion walk; the apply run removed 34 stale artifact dirs — the four Tauri targets plus 30 npm/venv trees — freeing +53 GB on D: (279 -> 332 GB); fresh+small entries stay untouched and CONTRIBUTING documents the widened classes; the scheduled task needed no change

## ✅ Done Index

> One-liner per completed item (details above).

- M107 — Pruning policy for depth-2 cargo targets and JS/Python artifacts — done (done (phase 84) — clean-targets.ps1 probes two levels per project (admitting Tauri's src-tauri\target) and covers cargo/node_modules/.venv with the stale rule; the root-caused -Depth-1 miss (depth 2 from the scan root) was replaced by targeted Test-Path probes, making the report run take 3 s instead of a full-recursion walk; the apply run removed 34 stale artifact dirs — the four Tauri targets plus 30 npm/venv trees — freeing +53 GB on D: (279 -> 332 GB); fresh+small entries stay untouched and CONTRIBUTING documents the widened classes; the scheduled task needed no change)
