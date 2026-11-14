# BACKLOG_ARCHIVE.p82.md — Shard (Phase 82, append-only)

> Created 2026-11-14T23:06 — home for item blocks archived in the context of Phase 82 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 82. Header is write-once — never edited.

---

### [x] M105 — Local unix gate via WSL with bounded build artifacts — 🟡 — 🟡
- **Location:** the verification ritual (AGENTS §4, PLAYBOOK §5) and the build-artifact homes: `target/` beside every workspace on D:, and — since phase 82 — `scripts/unix-gate.sh` driving a WSL Ubuntu toolchain.
- **Problem:** CI is the binding gate for `cfg(unix)` code, but the phase-80/81 runs failed on exactly the findings a local unix pass would have caught earlier (clippy `collapsible_if`, rustdoc links — commit `a719a43`). Separately, cargo build artifacts once filled drive D: to zero free space; both the Windows side and the WSL distro must stay bounded by construction, not by memory.
- **Approach:** `scripts/unix-gate.sh` runs the unix-side gates inside WSL Ubuntu with `CARGO_TARGET_DIR` on the WSL-native filesystem (never on a mounted Windows drive), reports the target size, and **cleans it afterwards by default** (`--keep` opts out) so nothing accumulates in the distro; the WSL VHDX is set sparse so deleted bytes actually shrink the disk file. The rustup install is `--profile minimal` plus clippy only. Windows-side, `CARGO_TARGET_DIR` points at the user profile so no workspace's `target/` can fill D: again; existing artifact directories are deleted only with the human's explicit confirmation.
- **Acceptance:** the gate runs end-to-end in WSL (clippy `--workspace --all-targets -D warnings` green), leaves `~/.cargo`/`~/.rustup` (the toolchain, by design) and no target directory behind, `wsl --manage Ubuntu --set-sparse true` succeeded, and the repo commit carries the script; old D: artifacts are reported, and only removed after the human says so.
- **Done:** done (phase 82) — scripts/unix-gate.sh: WSL-native target dir with an auto-clean default and a /mnt refusal, minimal rustup + clippy in WSL Ubuntu 26.04 (toolchain auto-synced from the repo pin); first run green (clippy --workspace --all-targets, 1m16s, peak 1.3G, cleaned); documented in CONTRIBUTING; Windows-side CARGO_TARGET_DIR redirected to the user profile (machine-level, outside the repo); the 99 GB D:\Development\datacraft\target is reported and awaits the human's deletion decision; sparse VHDX stays off (MS blocks it as unsafe) — the auto-clean bounds the distro instead

## ✅ Done Index

> One-liner per completed item (details above).

- M105 — Local unix gate via WSL with bounded build artifacts — done (done (phase 82) — scripts/unix-gate.sh: WSL-native target dir with an auto-clean default and a /mnt refusal, minimal rustup + clippy in WSL Ubuntu 26.04 (toolchain auto-synced from the repo pin); first run green (clippy --workspace --all-targets, 1m16s, peak 1.3G, cleaned); documented in CONTRIBUTING; Windows-side CARGO_TARGET_DIR redirected to the user profile (machine-level, outside the repo); the 99 GB D:\Development\datacraft\target is reported and awaits the human's deletion decision; sparse VHDX stays off (MS blocks it as unsafe) — the auto-clean bounds the distro instead)
