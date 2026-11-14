# BACKLOG_ARCHIVE.p83.md — Shard (Phase 83, append-only)

> Created 2026-11-14T23:12 — home for item blocks archived in the context of Phase 83 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 83. Header is write-once — never edited.

---

### [x] M106 — Automated target-dir pruning: keep only the last build — 🟡 — 🟡
- **Location:** the machine-level artifact homes — every `D:\Development\<project>\target` (99 GB in this workspace alone, 2026-11-14) and the redirected `CARGO_TARGET_DIR` (`C:\Users\sebas\.cargo-target`, set in M105) — automated by `scripts/clean-targets.ps1` and a weekly Windows scheduled task.
- **Problem:** cargo's stable `clean` is all-or-nothing and `gc` (age-based pruning) is nightly-only, so `target/` dirs grow without bound across builds — drive D: was once filled to zero free bytes. The user's policy: keep only the last build, automated, without leftovers on any drive.
- **Approach:** a PowerShell script enforces the policy per target directory: **stale** (no build for > MaxAgeDays, default 21) → remove the whole dir; **oversized** (fresh but > MaxSizeGB, default 20) → empty it, so the next build regenerates exactly the last build's artifacts; **fresh and small** → keep untouched. A directory only counts as a cargo target when its parent carries a `Cargo.toml` (name collisions are never touched) or when it is the global redirect dir itself. The scheduled task runs the apply mode weekly and logs to `%LOCALAPPDATA%\datacraft-clean-targets.log`; report mode is the default so a manual run never deletes unannounced.
- **Acceptance:** the report run names every found target with size and age; the apply run shrinks this workspace's 99 GB `target` under the threshold and keeps the recent-build artifacts of every fresh small project; the task is registered (`schtasks /Query` proves it) and removable with one documented command; CONTRIBUTING documents the policy.
- **Done:** done (phase 83) — scripts/clean-targets.ps1 enforces keep-only-the-last-build (stale > 21 d removed, fresh > 20 GB emptied, fresh+small kept; cargo-target detection via parent Cargo.toml plus the global CARGO_TARGET_DIR); the report run named the 102 GB datacraft target, the apply run emptied it (+99 GB free on D:), the weekly task 'DataCraft Clean Cargo Targets' (Sunday 03:00, StartWhenAvailable) is registered and test-fired with result 0, and CONTRIBUTING documents the policy and its removal command

## ✅ Done Index

> One-liner per completed item (details above).

- M106 — Automated target-dir pruning: keep only the last build — done (done (phase 83) — scripts/clean-targets.ps1 enforces keep-only-the-last-build (stale > 21 d removed, fresh > 20 GB emptied, fresh+small kept; cargo-target detection via parent Cargo.toml plus the global CARGO_TARGET_DIR); the report run named the 102 GB datacraft target, the apply run emptied it (+99 GB free on D:), the weekly task 'DataCraft Clean Cargo Targets' (Sunday 03:00, StartWhenAvailable) is registered and test-fired with result 0, and CONTRIBUTING documents the policy and its removal command)
