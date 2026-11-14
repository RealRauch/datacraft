#!/usr/bin/env bash
# unix-gate.sh — the unix-side quality gates inside WSL (item M105).
#
# CI (ubuntu-latest) is the binding gate for every `cfg(unix)` code path;
# this script is the local early catch so such findings surface before a
# push, not after a red run. It exists because a Windows checkout never
# compiles cfg(unix) code — phases 80/81 shipped two findings exactly
# there (commit a719a43).
#
# Disk discipline (the reason this script exists in this shape):
# - The build artifacts live on the WSL-native filesystem
#   ($HOME/.datacraft-target), NEVER on a mounted Windows drive — a
#   `target/` on /mnt/* once filled a Windows drive to zero bytes free.
# - The target directory is **removed after every run** unless --keep is
#   given: nothing is left behind in the distro by default. A repeated
#   run pays a full rebuild; that is the honest price of the policy.
# - The distro's VHDX is set sparse separately (wsl --manage …), so
#   deleted bytes actually shrink the disk file (see item M105).
#
# Usage (from Windows, Git Bash or any shell):
#   wsl -d Ubuntu -- bash -lc "cd /mnt/d/Development/datacraft && scripts/unix-gate.sh"
#   scripts/unix-gate.sh clippy   # the default: clippy only, the fast gate
#   scripts/unix-gate.sh tests    # clippy + the full workspace test suite
#   scripts/unix-gate.sh --keep   # keep the target dir for a fast repeat
#   scripts/unix-gate.sh clean    # remove the target dir, print sizes, exit
#
# What remains in the distro: ~/.cargo and ~/.rustup (the pinned
# toolchain, by design) and nothing else from a run.
set -euo pipefail

TARGET_DIR="$HOME/.datacraft-target"

# Self-sufficient PATH: rustup may be installed with --no-modify-path.
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
MODE="clippy"
KEEP=0
for arg in "$@"; do
    case "$arg" in
        clippy | tests) MODE="$arg" ;;
        --keep) KEEP=1 ;;
        clean) MODE="clean" ;;
        *)
            echo "unknown argument: $arg (use clippy|tests|--keep|clean)" >&2
            exit 2
            ;;
    esac
done

report_size() {
    if [ -d "$TARGET_DIR" ]; then
        printf 'target dir: %s (%s)\n' "$TARGET_DIR" "$(du -sh "$TARGET_DIR" | cut -f1)"
    else
        printf 'target dir: %s (absent)\n' "$TARGET_DIR"
    fi
}

if [ "$MODE" = "clean" ]; then
    report_size
    rm -rf "$TARGET_DIR"
    echo "target dir removed."
    report_size
    exit 0
fi

cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR="$TARGET_DIR"
# Guard the disk rule structurally: refuse to build onto a mounted
# Windows drive even if someone overrides CARGO_TARGET_DIR.
case "$CARGO_TARGET_DIR" in
    /mnt/*)
        echo "refusing: CARGO_TARGET_DIR is on a mounted Windows drive ($CARGO_TARGET_DIR)" >&2
        exit 2
        ;;
esac

echo "== unix gate ($MODE) on $(rustc --version) =="
if [ "$MODE" = "clippy" ]; then
    cargo clippy --workspace --all-targets -- -D warnings
else
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
fi
report_size

if [ "$KEEP" = 1 ]; then
    echo "kept the target dir (--keep); 'scripts/unix-gate.sh clean' removes it."
else
    rm -rf "$TARGET_DIR"
    echo "target dir cleaned (default policy: nothing stays behind)."
fi
