# BACKLOG_ARCHIVE.p94.md — Shard (Phase 94, append-only)

> Created 2026-11-22T00:11 — home for item blocks archived in the context of Phase 94 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 94. Header is write-once — never edited.

---

### [x] T12 — The i18n rule has no mechanical t()-bypass guard — 🔵
- **Location:** the CI hygiene job (`.github/workflows/ci.yml`) or an in-crate source-contract test over `crates/datacraft-ui/src/`; the gap evidenced by `crates/datacraft-ui/src/panes.rs` `pane_header` (step 93.1) — a visible pane title bypassed `t()` and no gate caught it.
- **Problem:** AGENTS §6's i18n rule ("UI strings go through `t()`/`tr()`") lives as prose only. The catalog parity tests cover key ↔ translation consistency, not bypass: a raw English literal handed straight to an egui render call compiles, renders English in every language, and passes the whole suite. The pane-header gap survived until the M125 acceptance review because the pilot's title ("SQL") is identical in German — a smooth fixture (LESSONS #7 instance).
- **Approach:** a mechanical scan at the moment the guard is defined (LESSONS #26): over the UI crate's paint sites, a static string literal may never reach an egui rendering call (`ui.label("…")`, `ui.button("…")`, `ui.strong("…")`, `Button::new("…")`, `Label::new("…")`, `menu_button("…")`, …) without going through `t()`/`tr()`; dynamic (`format!`-built, variable, `&'static str`-key) arguments stay allowed, with an explicit, minimal painted-literal allowlist (the glyph-coverage test's precedent, T11). Ride the existing CI hygiene job so the gate is mechanical and visible.
- **Acceptance:** a raw literal in an egui render call fails the gate; the allowlist is explicit and registered where the scan lives; the panes.rs regression class cannot return unnoticed.
- **Done:** RED 9802391 (tree scan fails on 8 deliberate paint literals), guard 5b2ae3c (allowlist + self-tests), CI wiring 3796001 (hygiene job rides the scan, high gate released)

## ✅ Done Index

> One-liner per completed item (details above).

- T12 — The i18n rule has no mechanical t()-bypass guard — done (RED 9802391 (tree scan fails on 8 deliberate paint literals), guard 5b2ae3c (allowlist + self-tests), CI wiring 3796001 (hygiene job rides the scan, high gate released))
