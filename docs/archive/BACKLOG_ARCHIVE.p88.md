# BACKLOG_ARCHIVE.p88.md — Shard (Phase 88, append-only)

> Created 2026-11-16T16:36 — home for item blocks archived in the context of Phase 88 (rotation: last active phase, H14/34.2).
> Contains: items archived during Phase 88. Header is write-once — never edited.

---

### [x] M98 — Ribbon has no group overflow at narrow widths — 🟡
- **Location:** `crates/datacraft-ui/src/ribbon.rs` (group row) and the spec in `crates/datacraft-core/src/ribbon.rs`; H52 sized the groups to content and T10 pins them in-window at 1280x800 and 960x600.
- **Problem:** There is no overflow rule below those widths: groups keep their natural widths, so with a narrower window (or a longer localized label) trailing groups leave the window and their actions become unreachable. Access collapses groups into dropdown buttons; craft-libs' `layout::fit_toolbar` documents the policy (keep whole groups, reserve the overflow control's width, never let a command disappear silently).
- **Approach:** Keep the layout math in core (the H52 precedent): compute visible groups plus overflow indices from natural widths, available width, spacing and the overflow control's width. The UI paints one overflow control whose menu dispatches the same `command_for` actions, so disabled states and tooltips stay identical. Decide per group whether to collapse or to list the actions in the overflow menu.
- **Acceptance:** At 800x600 every ribbon action stays reachable (visible or in the overflow list); layout_probe/kittest pins the group widths and the overflow boundary at 800x600, 960x600 and 1280x800 in both languages; no command disappears without a visible entry point.
- **Steps:** none yet.
- **Done:** done in `93fc293` (core fit_groups math, RED `67b2c8b`) + `576eefd` (UI overflow control, RED `4a37401`) — step 88.1: the core math answers a GroupFit plan (whole groups, control width reserved first, total for degenerate rows); the UI paints one localized overflow control whose menu stacks the hidden groups' own action buttons, so dispatch, disabled states and tooltips are identical by construction; pins cover the boundary at 1280x800/960x600/800x600 in both languages, menu reachability at narrow widths and menu dispatch equalling the row command

---

### [x] L45 — Tab strip: the selected tab can scroll out of view — 🟢
- **Location:** `crates/datacraft-ui/src/workspace.rs:46` (the horizontal `ScrollArea` around the tab row).
- **Problem:** The tab strip has no fitting or scroll-to-selected rule: with many open objects the active tab can sit outside the visible strip, and activation from the nav pane, a command or a lifecycle action does not scroll it back into view. craft-libs' `tabs::fit`/`fit_whole` document the policy (shrink wide tabs before hiding them, always keep the selected tab visible, overflow the rest).
- **Approach:** Keep the current strip and add a scroll-to-selected rule on activation; only if that proves insufficient, add a width policy (shrink wide tabs, overflow list) with the math in core. No dependency.
- **Acceptance:** Activating an off-screen tab scrolls the strip to it (kittest/layout_probe pin); every tab stays reachable by pointer and keyboard; the selected tab is always visible after nav-pane activation and lifecycle-driven activation.
- **Steps:** none yet.
- **Done:** done in `c7e60b7` (RED `c0e5ad9`) — step 88.2: the strip row lives in a horizontal ScrollArea and a scroll-to-selected rule fires on activation only; every tab stays reachable by pointer (scrollable content, pinned) and by keyboard (all activation paths manifest as a changed active tab); the pins cover the overflowing strip at 640x600 with the lifecycle-opened and the nav-pane re-activated tab fully visible

---

### [x] L49 — Tabs: no close-to-side and no reuse of an idle tab — 🟢
- **Location:** the tab context menu and `CloseOtherTabs`/`CloseAllTabs` (U10), and the open-object paths that always create a new tab.
- **Problem:** The context menu closes a tab, the others or all, but not the tabs to the left/right; and every activation opens another tab even when the same object is already open and idle. The reference client reuses an idle tab (no pending changes) for navigation targets and closes to either side.
- **Approach:** Two small additions: Close to the Left/Right as bus commands (additive wire vocabulary, same shape as U10) and a reuse policy in the open-object path (activate the existing tab when the object is already open in the requested mode and has no pending edits; H61 introduces that state). No new UI surface beyond two context-menu entries.
- **Acceptance:** Close to the Left/Right closes exactly the expected tabs and keeps the active one; activating an already-open idle object activates its tab instead of opening a second one; an object with pending edits is never replaced; JSON-RPC pins for the two new commands.
- **Steps:** none yet.
- **Done:** done in `483ce35` (core commands + reuse fix, RED `86be08b`) + `3022ab4` (UI menu, RED `f60ca45`) — step 88.3: CloseTabsToLeft/Right anchor on one surviving tab, move activation only when it was on the closed side, answer empty-side and unknown-tab precisely and round-trip over the JSON-RPC wire; the reuse lookup now matches case-insensitively (the tab_reuse pin caught 'ORDERS' opening a duplicate tab of the open 'Orders'); the reuse/pending-edit pins ran green on arrival (documented no-RED) and the menu kittest proves a staged batch survives re-activation

---

### [x] T11 — Glyph coverage of the shipped UI fonts is untested — 🔵
- **Location:** `crates/datacraft-ui/src/theme/fonts.rs` (Inter and its named cuts in front of egui's defaults), the de/en catalogs under `crates/datacraft-ui/src/i18n/`, and every painter that draws a glyph instead of typing it.
- **Problem:** egui draws a character no installed face covers as tofu; the project already hit two cases (F12 combo caret, H53 ✕ U+2715) and fixed them by painting instead of typing. Nothing mechanically checks that every character the UI can render — catalog strings, database/object/file names, formatted values — has a glyph in the shipped stack, so the next missing character reaches users unnoticed.
- **Approach:** A test that walks both catalogs plus a sample of hostile names (CJK, emoji, accents, symbols) and asks the installed `FontDefinitions` (or a small cmap reader over the embedded faces) whether a glyph exists; it fails naming the language, key and code point. Intentionally painted glyphs get an explicit allowlist so the check does not flag them. craft-libs' `fonts::assert_covers` is the reference.
- **Acceptance:** The test fails on an injected missing character (RED evidenced before the GREEN), passes on the shipped stack, names language/key/code point, and runs headless without a GPU.
- **Steps:** none yet.
- **Done:** done in `00ca6ea` (RED `e882763`) — step 88.4: the coverage test walks every English key and German value of both catalog scopes plus a hostile-name sample against the installed egui font stack headless (FontsView::has_glyph, one Context, no GPU); the RED is an injected U+2715 string failing with language, key and code point named; the accepted-missing allowlist carries only the genuinely missing CJK/emoji code points, each pinned as missing so coverage growth removes it

## ✅ Done Index

> One-liner per completed item (details above).

- M98 — Ribbon has no group overflow at narrow widths — done (done in `93fc293` (core fit_groups math, RED `67b2c8b`) + `576eefd` (UI overflow control, RED `4a37401`) — step 88.1: the core math answers a GroupFit plan (whole groups, control width reserved first, total for degenerate rows); the UI paints one localized overflow control whose menu stacks the hidden groups' own action buttons, so dispatch, disabled states and tooltips are identical by construction; pins cover the boundary at 1280x800/960x600/800x600 in both languages, menu reachability at narrow widths and menu dispatch equalling the row command)
- L45 — Tab strip: the selected tab can scroll out of view — done (done in `c7e60b7` (RED `c0e5ad9`) — step 88.2: the strip row lives in a horizontal ScrollArea and a scroll-to-selected rule fires on activation only; every tab stays reachable by pointer (scrollable content, pinned) and by keyboard (all activation paths manifest as a changed active tab); the pins cover the overflowing strip at 640x600 with the lifecycle-opened and the nav-pane re-activated tab fully visible)
- L49 — Tabs: no close-to-side and no reuse of an idle tab — done (done in `483ce35` (core commands + reuse fix, RED `86be08b`) + `3022ab4` (UI menu, RED `f60ca45`) — step 88.3: CloseTabsToLeft/Right anchor on one surviving tab, move activation only when it was on the closed side, answer empty-side and unknown-tab precisely and round-trip over the JSON-RPC wire; the reuse lookup now matches case-insensitively (the tab_reuse pin caught 'ORDERS' opening a duplicate tab of the open 'Orders'); the reuse/pending-edit pins ran green on arrival (documented no-RED) and the menu kittest proves a staged batch survives re-activation)
- T11 — Glyph coverage of the shipped UI fonts is untested — done (done in `00ca6ea` (RED `e882763`) — step 88.4: the coverage test walks every English key and German value of both catalog scopes plus a hostile-name sample against the installed egui font stack headless (FontsView::has_glyph, one Context, no GPU); the RED is an injected U+2715 string failing with language, key and code point named; the accepted-missing allowlist carries only the genuinely missing CJK/emoji code points, each pinned as missing so coverage growth removes it)
