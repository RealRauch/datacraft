# BACKLOG_ARCHIVE.p91.md — Shard (Phase 91, append-only)

> Created 2026-11-19T20:24 — home for item blocks archived in the context of Phase 91 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 91. Header is write-once — never edited.

---

### [x] M109 — No version display in any client (UI, CLI, MCP) — 🟡
- **Location:** the CLI entry (`crates/datacraft-cli/src/main.rs`, `crates/datacraft-cli/src/lib.rs` argument handling), the MCP stdio endpoint (`crates/datacraft-mcp`), the backstage Info page (`crates/datacraft-ui/src/backstage/`), one shared version source in the workspace.
- **Problem:** Found in the L47 design review (2026-11-16, finding 1 in `docs/UI-DESIGN-REVIEW.md`): no client can report its version. The CLI has no `--version` flag (archived L34 noted the flag "does not exist in the CLI vocabulary"), the command vocabulary has no About/Version command, and the backstage Info page shows no version. A user or agent cannot tell which build it is talking to; bug reports and MCP sessions lack the version anchor.
- **Approach:** One source of truth — the workspace version (`version.workspace = true` makes every crate's `CARGO_PKG_VERSION` identical). Surface it where each client answers: CLI `--version` (prints `datacraft-cli <version>`, exit 0, before any database work) and a `version` word in the REPL if one fits the existing vocabulary; MCP reports the same string in its server/initialize information (additive, no wire break); the backstage Info page shows it (label localized via `t()`, the version value stays locale-neutral). No new dependency, no new heavyweight command — reuse the existing info/inventory surface if one fits.
- **Acceptance:** `--version` exits 0 with name + version on stdout; the three clients (CLI, MCP wire, backstage Info page) report the identical version string, pinned by one contract test over a shared source; the Info page string goes through `t()`; kittest pins the Info page; MCP wire test pins the report; usage-error and exit-code contracts unchanged.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `141bd90` — one shared source `datacraft_core::VERSION` (workspace lockstep): CLI `--version`/`-V` exits 0 printing `datacraft-cli <version>` on stdout before any database work (position-independent); the wire gains the additive `version` method (name + version, no existing contract moves); the backstage Info page shows the row above the database facts, with and without an open database; contract pins in transport/cli_contract/ui_kittest_version over the shared const; de catalog entry added

## ✅ Done Index

> One-liner per completed item (details above).

- M109 — No version display in any client (UI, CLI, MCP) — done (done in `141bd90` — one shared source `datacraft_core::VERSION` (workspace lockstep): CLI `--version`/`-V` exits 0 printing `datacraft-cli <version>` on stdout before any database work (position-independent); the wire gains the additive `version` method (name + version, no existing contract moves); the backstage Info page shows the row above the database facts, with and without an open database; contract pins in transport/cli_contract/ui_kittest_version over the shared const; de catalog entry added)
