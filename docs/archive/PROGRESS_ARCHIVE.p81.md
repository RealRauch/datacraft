# PROGRESS_ARCHIVE.p81.md — Shard (Phase 81, append-only)

> Created 2026-11-14T21:19 — home for the detail block of Phase 81 on completion.
> Contains: Phase 81 — Queued native dialogs. Header is write-once — never edited.

---

### Phase 81 — Queued native dialogs

**Scope (Steps):**

- **81.1 Dialog seam rework: DialogRequest/DialogAnswer queue, ScriptedDialogs through the same queue, native host on rfd's async API (M97, RED first)**
- **81.2 Call-site migration: parked intents for the ten sites, one-pending guard, per-frame answer poll (M97)**
- **81.3 Documentation and phase gate: COMPATIBILITY macOS-unverified note, kittest flows green, M97 archive**
**Verification:** Gate review of phase 81: the re-scoped M97 delivered through the RED→GREEN chain (81.1 seam, 81.2 migration); the full verification set is green (cargo test --workspace, clippy -D warnings, fmt, file-size gate — app_tests.rs split at 548); M97 archived with the honest unverified-on-macOS-hardware marker in docs/COMPATIBILITY.md; no dependencies touched (rfd's async API was already in the tree), no CI touched (all Low class).
