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

---

### [x] M110 — SQL window grows animated toward full screen height on every run — 🟡
- **Location:** `crates/datacraft-ui/src/datasheet/grid.rs` (the shared grid's `ScrollArea::both().auto_shrink([false, false])` at the `scroll_area` construction) plus `crates/datacraft-ui/src/sql_window.rs` (embeds `grid_ui` inside the floating `egui::Window` without a height budget).
- **Problem:** Found in the L47 design review (2026-11-16, finding 2 in `docs/UI-DESIGN-REVIEW.md`, reviewer screenshot): running any statement stretches the SQL window animated toward full screen height — "as if the query were still producing rows" — with four result rows on screen. Mechanism: `auto_shrink([false, false])` claims all available space in both axes (correct inside the datasheet's CentralPanel), but inside a resizable `egui::Window` the claim is circular — the window auto-sizes to its content while the content claims the available size — and `show_rows` supplies the virtual height (rows × row_height) into that loop. egui's window-size interpolation turns the per-frame growth into the visible animation.
- **Approach:** A height policy on `grid_ui`/the grid host: fill (the datasheet host, unchanged) vs. an explicit bounded height for window embedders (the SQL window passes a result height; the window then keeps the user's resized size). Alternative: constrain the window's result container instead of the grid — decided at implementation, one of the two must hold: the datasheet host keeps `auto_shrink([false, false])`, the window embedder does not feed the size loop.
- **Acceptance:** Running a 4-row query keeps the SQL window at its current size (no growth animation); the window stays user-resizable and large result sets scroll inside the bounded result area; the datasheet's layout_probe pins are unchanged at both pinned window sizes; kittest pins result visibility in the SQL window; no regression in the other `grid_ui` consumer (the datasheet).
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `4b532b1` — the shared grid's `max_height` policy: None keeps the datasheet/tab-hosted grids filling (pins unchanged), Some(bound) shrinks to content and caps — the SQL window passes a ten-row bound so the window's auto-size loop is no longer fed; the window records its rect under the `sql.window` layout probe; frame-stability discriminator proves the fix (policy off: window grows 539→709 px between frames 5–15, both tests red)

---

### [x] M111 — Import File accepts database files and answers with a raw decoder error — 🟡
- **Location:** `crates/datacraft-core/src/import.rs` (`read_csv_file` → `decode_csv_text`), the import flow (`crates/datacraft-ui/src/import_window.rs`, `PreviewImportFile`/`ImportFile`), and the file picker.
- **Problem:** Found in the L47 design review (2026-11-16, finding 3 in `docs/UI-DESIGN-REVIEW.md`, reviewer screenshot): a Jet4 `.mdb` chosen in the Import File window (the delimited-text import surface) runs the text-decoder chain (UTF-8 → UTF-16 BOM → Windows-1252) and answers "byte 0x00 at offset 0 is not defined in Windows-1252" — precise but misleading: the file is a healthy database on the wrong surface, and nothing names the right surface. The composite's format sniff already identifies the file as Jet4.
- **Approach:** Before decoding, run the existing format-detection knowledge over the chosen source: a database family (Jet3/Jet4/ACE/SQLite) answers a precise error naming the detected format and the right surface ("open it via File → Open"); the picker filters to the supported text/CSV extensions first. Reuse the sniff — no new detection logic, no new dependency.
- **Acceptance:** Importing a Jet4 fixture through `PreviewImportFile`/`ImportFile` answers the format-naming error over the wire and in the UI (pinned), not a decoder byte error; genuine text/CSV imports behave unchanged; the picker filter names the supported extensions; the SQLite and Jet3 families answer the same way (one test per family or one parameterized).
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `f1e5e45` — read_csv_file sniffs the head before the decode chain (core `port::sniff_database_family`, the registry's signatures, no new dependency): Jet/ACE and SQLite files answer "this is a <family> database file, not delimited text — open it via File → Open"; the open-file dialog request carries the surface's extensions (import csv/txt/tsv, open mdb/accdb/sqlite/sqlite3/db) through the localized "Supported files" filter; wire pins over both families, kittest pin in the window flow

---

### [x] M112 — Import errors repeat the file path twice — 🟡
- **Location:** `crates/datacraft-core/src/import.rs` (`read_csv_file` wraps the decode error with `format!("{}: {e}", path.display())`) and the inner decode layer whose error already carries the path.
- **Problem:** Found in the L47 design review (2026-11-16, finding 4, reviewer screenshot): the status line shows `C:\…\database-access-ready.mdb: C:\…\database-access-ready.mdb: byte 0x00 …` — two error layers prepend the same path, pushing the actual reason off-screen.
- **Approach:** One layer owns the path prefix: the inner decode error stays path-free, the outer `read_csv_file` adds `{path}: ` exactly once. Error text is contract — pinned by tests, not by string tricks.
- **Acceptance:** A failing import shows the path exactly once in the composed error; the precise inner locations survive unchanged (`row N:`, `header column …`); a test pins the composed text for the decode-failure case.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `f1e5e45` — one layer owns the path prefix: read_csv_file keeps its single `{path}: ` wrapper (both the read and the decode branch), and the import_file/preview_import_file handlers forward the reason untouched; the composed decode error carries the path exactly once, pinned with the precise inner location (import_surface.rs)

---

### [x] L53 — Import source-file field clips long paths at the start — 🟢
- **Location:** `crates/datacraft-ui/src/import_window.rs` (the Quelldatei/source-file row: field width and the window's default size). Folded 2026-11-19: the same fixed-width path fields across the backstage — Open (`backstage/pages.rs` `open_page`), New (`backstage/new_database.rs` `location_step`), Save-As and sidecar (`backstage/save.rs`), each `desired_width(360.0)`.
- **Problem:** Found in the L47 design review (2026-11-16, finding 6, reviewer screenshot): realistic long paths show only their tail in the narrow field — the directory context at the start is invisible; the window is resizable but the default is too small. The 2026-11-19 review pass found the same fixed-width pattern in every backstage path field, so one path-field contract is folded here instead of per-field items.
- **Approach:** One contract for all path fields: wider field (window default width or field proportion) and the chosen file name shown prominently once picked (the full path on hover). Simplest sufficient variant at implementation; no functional change to any flow.
- **Acceptance:** A realistic long path (`C:\Users\…\Downloads\database-access-ready.mdb`) is identifiable at the default size in the import window and in every backstage path field — the file name is visible and the full path rides the hover; kittest pins the fields; import/open/create/save behaviour unchanged.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `f1e5e45` — one path-field contract: labels::path_field_identity renders the chosen file name as its own strong row with the full path on hover, beside the import source field and every backstage path field (open, new-database location, save-a-copy, sidecar); fields widened (import 280→360 at the 520 default window, backstage 360→480); kittest pins all five surfaces at the default size

---

### [x] L52 — Status-line errors flood the bar without truncation — 🟢
- **Location:** `crates/datacraft-ui/src/shell.rs` (the status bar's error/notice rendering, `notice_label`) and, folded 2026-11-19, `crates/datacraft-ui/src/backstage/pages.rs` (`status_line` — the same unbounded last-error/notice text at the backstage page bottoms).
- **Problem:** Found in the L47 design review (2026-11-16, finding 5 in `docs/UI-DESIGN-REVIEW.md`, reviewer screenshot): a long error paints the entire status bar — the click-to-open-history affordance exists, but the inline text has no ellipsis/hover contract and floods the chrome. The 2026-11-19 review pass (surfaces closed) found the same unbounded text pattern in the backstage `status_line`, so the truncation contract is folded here instead of a second item.
- **Approach:** Truncate the inline text to the available width with an ellipsis at both call sites (ideally one shared helper); the full text rides the hover tooltip; the existing click opens the message history unchanged. Truncation is pure width math over the chrome font metrics.
- **Acceptance:** A long error truncates with an ellipsis, hover shows the full text, click still opens the history — on the status bar and on the backstage pages; layout_probe pins the truncated rects at both pinned window sizes; the string path stays in `t()`/`tr()` where localized.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `64da362` — one shared truncation helper (shell::notice_label, crate-visible): binary-searched ellipsis clip over the label font's width math, full text above the click hint on hover, click-to-history unchanged; the status bar and the backstage status line share it; label rect recorded under the `status.notice` layout probe, pinned at both pinned window sizes

---

### [x] M113 — Sort order scrambles with NULL values and the indicator can claim unapplied sorts — 🟡
- **Location:** `crates/datacraft-types/src/compare.rs` (`compare` returns `None` for NULL and mixed-family pairs), `crates/datacraft-core/src/table_view.rs` (`view_order` degrades `None` → `Ordering::Equal`, an inconsistent comparator), `crates/datacraft-ui/src/datasheet.rs` (`.ok()` fallback on `view_order` errors while the sort indicator renders from `tab.sort_field` independently).
- **Problem:** Found in the L47 design review (2026-11-17, finding 7 in `docs/UI-DESIGN-REVIEW.md`, reviewer screenshot, "Sortierreihenfolge überprüfen"): a DateTime column with one NULL row sorted descending shows the descending indicator while the rows land in a scrambled order (dates 1937, 1923, 1935, …, 2026; ids 2, 1, 3, 4, 6, 7). Mechanism: `None`-comparisons become `Equal`, so the sort comparator violates transitivity and the stable sort produces an implementation-defined scramble; separately, a `view_order` error (unknown/renamed field) is swallowed and the indicator still claims a sort that is not applied.
- **Approach:** Define NULL placement per the Access convention (NULLs first ascending, last descending — documented in the behaviour docs and the compatibility matrix row for sort semantics), make the ordering comparator total (NULL-aware ordering function in the sort path instead of `Equal`-degradation; `compare` itself stays family-safe `Option` for its existing callers), and stop the silent fallbacks: a `view_order` error either surfaces as a precise notice or clears the sort state — never indicator-without-sort. Mixed-family pairs keep a defined, documented order (family rank) instead of `Equal`.
- **Acceptance:** `view_order` over DateTime/Text/Numeric columns with NULLs yields a defined, unit-tested order in both directions (NULL placement per the documented convention, stable for equal keys); the comparator passes a consistency property (no scrambled output on NULL mixes); a renamed sort field answers a precise notice or clears the indicator (pinned); the datasheet contract test pins indicator ↔ order agreement; docs name the NULL and mixed-family placement.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `7e35031` — the total comparator `datacraft_types::sort_compare` (NULLs first ascending/last descending via the direction reverse, declaration-order family rank under mixes, non-finite floats tiered after their family; `compare` stays family-safe Option for its callers), `view_order` sorts with it; the stale-sort fallback is gone — a view_order failure clears the sort state through `SetTableSort`-None (whose clear accepts any field name), so the indicator never claims an unapplied order; semantics documented in FEATURES and a COMPATIBILITY matrix row; core fix-forward `86c6452` corrected the property's direction-aware rank assertion

---

### [x] M114 — The AutoNumber cell's contract is invisible until a rejection — 🟡
- **Location:** the new-record hint (`crates/datacraft-ui/src/i18n/de/views.rs` — "Empty cells are Null; AutoNumber assigns itself." / „Leere Zellen sind Null; AutoWert vergibt sich selbst."), the datasheet edit-start path (`crates/datacraft-ui/src/datasheet.rs`, `row_edit_zls.rs`), the Jet rejection of user-supplied AutoNumber values (M8, `crates/datacraft-jet`).
- **Problem:** Found in the L47 design review (2026-11-17, finding 8, reviewer comment): the hint does not say whether a manual value may be entered. It may not — the Jet port rejects user-supplied AutoNumber values — and the datasheet has no AutoNumber edit gate, so the contract surfaces only as a runtime rejection after typing into the id cell.
- **Approach:** Make the contract visible before input: either the AutoNumber cell is not editable (the availability/capability gate pattern the datasheet already uses) or the hint states the rejection precisely ("Eingaben werden abgelehnt — der Motor vergibt den Wert"). The rejection error names the column. The chosen variant lands in both catalogs.
- **Acceptance:** The AutoNumber cell shows its contract before input (gate or precise hint, pinned by kittest); typing into it is either impossible or produces the precise rejection naming the column; en/de catalog parity; no change to the M8 writer behaviour itself.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `df7de1e` — the gate variant: engine-assigned new-record cells render disabled with the rejection reason on hover (probe `datasheet.newrow.autonumber_gate`), the row hint names the contract precisely in both catalogs, and the row_edit rejection names the column; the M8 writer behaviour untouched; the trailing row sits below the a11y fold headless, so the string pin rides a crate unit test and the gate rides the probe

---

### [x] L54 — "Felder fixieren" has no explanation and no visible frozen state — 🟢
- **Location:** the column context menu (Freeze Fields / Unfreeze All Fields — `crates/datacraft-ui/src/i18n/de/ui_chrome.rs`, "Freeze Fields"/"Felder fixieren" and "Unfreeze All Fields"/"Fixierung aufheben"), the grid's frozen-column rendering (`crates/datacraft-ui/src/datasheet/grid.rs`, `effective_frozen`).
- **Problem:** Found in the L47 design review (2026-11-17, finding 9, reviewer comment): it is unclear what "Felder fixieren" does — no tooltip explains that the leading columns stay pinned during horizontal scroll, and the frozen state itself carries no visible marker in the grid beyond the layout change.
- **Approach:** A one-line tooltip on both menu items (what freezing does), and a visible frozen-state indicator in the grid (e.g. a stronger separator after the last frozen column and/or a check/state mark on the menu items); i18n parity en/de; no behaviour change to the freeze itself (M39 semantics stay).
- **Acceptance:** Both menu items carry tooltips; the frozen state is recognisable without opening the menu (layout_probe pin at both pinned window sizes); "Fixierung aufheben" returns the plain grid; kittest pins menu and state.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `df7de1e` — both freeze menu items carry one-line tooltips ("Keep the leading columns pinned…", "Release the pinned columns."), and the frozen state is visible without the menu: a 2 px accent boundary after the last frozen column in header and body, recorded under the `datasheet.frozen_boundary` layout probe and pinned present frozen / absent unfrozen; M39 semantics unchanged

---

### [x] M115 — Nav-pane sort menu shows no active order and reads per-group while being global — 🟡
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (the group-header context menu's sort entries; the order itself is the global `app.nav_sort` applied by `nav::sort_entries` to every section).
- **Problem:** Found in the L47 design review (2026-11-19, finding 11 in `docs/UI-DESIGN-REVIEW.md`): the group-header menu offers "Sort by Name"/"Sort by Type" as plain buttons with no indicator of the active order, and the same menu sits on every group header while it sets one pane-wide order — the affordance reads per-group but acts global.
- **Approach:** Render the active order as a check/state mark on the menu entries (the M79 pattern: state-bearing controls render their state from the snapshot/state), and make the scope honest: label the entries as the pane-wide order ("Sort all sections by …") or scope the menu per section — one decision, applied consistently.
- **Acceptance:** The sort menu shows the active order (kittest pin); the entry labels name the order's scope truthfully; sorting behaviour itself is unchanged (pure presentation/labelling); en/de catalog parity.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `fa378a5` — the sort menu renders the active order as selectable state (the M79 pattern: name/type entries carry selected/unselected marks, pinned via the accesskit toggled property) and the entries name the pane-wide scope ("Sort all sections by Name/Type"); sorting behaviour unchanged; de catalog follows

---

### [x] L55 — Nav-pane entry click target excludes the icon — 🟢
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (entry rows: icon widget and `selectable_label` as separate widgets with the 6 px item spacing between them).
- **Problem:** Found in the L47 design review (2026-11-19, finding 12): only the label opens an object — the icon is inert, and icon+label read as two widgets instead of one row; Access opens from the whole row.
- **Approach:** Make the whole row one click target (allocate the row, paint icon and label inside, interact over the full rect — the backstage nav-row pattern), keeping the context menu, active state and test ids.
- **Acceptance:** Clicking anywhere on a row (icon included) opens the object (kittest); context menu, selection state and automation ids unchanged.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `fa378a5` — a nav row is one interacted rect (the backstage nav-row pattern): icon and label paint inside it, the row carries the selected widget state, and a click over the icon area opens the object (pinned by a geometric icon-position click); context menu, selection state and automation ids unchanged

---

### [x] L56 — Native-objects inventory renders as disabled buttons — 🟢
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (the native forms/reports group: `ui.add_enabled(false, egui::Button::new(name))` per entry).
- **Problem:** Found in the L47 design review (2026-11-19, finding 13): the read-only inventory renders every entry as a disabled button while all other entries are selectable labels — the group reads as a block of dead controls.
- **Approach:** Render the entries in the pane's row language (icon + label like the other groups), carrying the read-only state through the group hint and muted text instead of button chrome.
- **Acceptance:** Native entries visually match the pane's row language (kittest pin); the read-only hint stays; no affordance appears where none exists.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `fa378a5` — the native inventory renders in the pane's row language (muted label row with its kind icon, never disabled button chrome); the row is a real Label widget so the name survives the semantic tree's static-text culling (the M60/M61 note — pinned with a tall-viewport harness); the read-only state carries the group hint and no affordance appears

---

### [x] L57 — The nav-pane category dropdown opens a menu that does nothing — 🟢
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (the "All Access Objects" menu button with its single selectable entry).
- **Problem:** Found in the L47 design review (2026-11-19, finding 14): the category dropdown's only menu entry re-states the current selection and performs nothing — a dead-looking affordance (LESSONS #29 family) until the category model grows (L48).
- **Approach:** Until real categories exist (L48), render the header as a plain labelled header without the empty menu, or keep the dropdown but disable its single entry with an explanatory tooltip; revisit when L48 lands.
- **Acceptance:** No menu opens that cannot do anything (kittest pins the header form); the Access category language returns with L48's categories.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `fa378a5` — the category header is a plain strong labelled header until the category model grows (L48); no menu opens that cannot do anything (LESSONS #29 family); the Access category language returns with L48

---

### [x] L58 — The nav pane's "New View" button sits detached at the pane bottom — 🟢
- **Location:** `crates/datacraft-ui/src/nav_pane.rs` (the `small_button` rendered below the scroll area).
- **Problem:** Found in the L47 design review (2026-11-19, finding 15): the button pins below the object list at the pane bottom regardless of any Views group state — an odd duplicate of the ribbon's Views entry point that reads as orphaned chrome.
- **Approach:** Move the create affordance into the Views group header's context menu (where the other group actions live) or beside the group header; the ribbon split-button stays as-is.
- **Acceptance:** The pane bottom carries no detached button; view creation remains reachable from the pane (kittest flow pins the new location).
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `fa378a5` — view creation lives in the Views group header's context menu (below the sort entries); the detached pane-bottom button is gone, and the menu entry opens the view editor (pinned via the open draft)

---

### [x] M116 — The Save page mixes titled and untitled sections — 🟡
- **Location:** `crates/datacraft-ui/src/backstage/save.rs` (`save_page`: the sidecar block starts with a bare field label; "Save a Copy" and "Database Password" carry `ui.heading`).
- **Problem:** Found in the L47 design review (2026-11-19, finding 16): the Save page has three sections, only two carry headings — the page reads unstructured and the sidecar section has no name to anchor it.
- **Approach:** Give every section a consistent heading treatment (e.g. "Design Sidecar", "Save a Copy", "Database Password"); heading rung and spacing follow the page grammar used elsewhere in the backstage.
- **Acceptance:** All three Save-page sections carry consistent headings (kittest pins them); en/de catalogs complete; no flow change.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ba97071` — the sidecar section carries the "Design Sidecar" heading (de: Design-Seitendatei) like the other two Save-page sections; no flow change; pinned by the backstage kittest (all three headings present)

---

### [x] L59 — The Info page's fact values do not align into a column — 🟢
- **Location:** `crates/datacraft-ui/src/backstage/info.rs` (the facts and driver-fact rows: label+value horizontals).
- **Problem:** Found in the L47 design review (2026-11-19, finding 17): label widths differ, so values start at varying x — the page reads ragged; a property-grid alignment would match the Access Info look.
- **Approach:** Two-column alignment: a fixed label column (or a max-label-width pass over the fact set) before the values.
- **Acceptance:** All fact values share one left edge at both pinned window sizes (layout_probe pin); long values wrap within the value column.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ba97071` — the Info page's fact rows align into a property grid: the widest label measures the shared column edge (strong-render slack absorbed), every value starts there; the value rects ride the `backstage.info.fact_value` layout probe, pinned equal-left at render time; driver facts, the integrity verdict and the lock-file row share the helper

---

### [x] M117 — The Relationships window has no scroll area — 🟡
- **Location:** `crates/datacraft-ui/src/relationships.rs` (window body: relationship list, create form and painted canvas render without a `ScrollArea`).
- **Problem:** Found in the L47 design review (2026-11-19, finding 20): once the database holds more relationships/tables than fit one window height, the list and the canvas clip with no way to reach the rest — the mapper silently loses content.
- **Approach:** Wrap the body in a vertical `ScrollArea` with a height budget (the form runtime's navigation-row reservation is the pattern); the deterministic canvas grid layout stays.
- **Acceptance:** A database with many tables/relationships scrolls to every list entry and every canvas box (kittest at both pinned window sizes); canvas geometry math unchanged.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — the mapper's body scrolls within a 380 px height budget; the viewport rect and every row rect ride the layout probe (`relationships.viewport` ≤ budget, all fixture rows laid out inside the scrollable body); canvas geometry math unchanged

---

### [x] M118 — VBA viewer paints error text in the literal LIGHT_RED instead of tokens.danger — 🟡
- **Location:** `crates/datacraft-ui/src/vba_viewer.rs` (two sites: the extraction-failure text and the analysis-error pane, both `egui::Color32::LIGHT_RED`).
- **Problem:** Found in the L47 design review (2026-11-19, finding 22): error text uses color literals, breaking the token contract (ADR-6: paint sites name roles, never literals) and rendering harshly in light mode and wrongly in dark mode, where every other error uses `tokens.danger`.
- **Approach:** Replace both literals with `crate::theme::tokens(ui.visuals()).danger` — the treatment every other error text gets.
- **Acceptance:** No color literals remain in the viewer; error text follows `tokens.danger` in both palettes (token/contract test or explicit review pin); rendering otherwise unchanged.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — both error sites paint `tokens.danger` instead of the LIGHT_RED literal; pinned as a source contract over the viewer's source (no color literals); rendering otherwise unchanged

---

### [x] M119 — The view editor's fields have no label association — 🟡
- **Location:** `crates/datacraft-ui/src/views_window.rs` (`view_editor`: the "View name" and "SQL" labels and their fields).
- **Problem:** Found in the L47 design review (2026-11-19, finding 23): neither field is associated with its label (`labelled_by`), unlike the established pattern (expression window, backstage open field) — screen readers and the kittest's accessibility checks see anonymous fields.
- **Approach:** Associate both fields with their labels via `labelled_by`, exactly like the neighbouring windows.
- **Acceptance:** Both editor fields carry their label association (kittest accessibility pass); no behaviour change.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — both view-editor fields carry their labels via labelled_by; the accessibility names resolve in the kittest tree (no behaviour change)

---

### [x] L60 — The expression window's Close button sits mid-title-row — 🟢
- **Location:** `crates/datacraft-ui/src/expression_window.rs` (the first row: the field label followed directly by Close).
- **Problem:** Found in the L47 design review (2026-11-19, finding 21): Close renders directly after the "Expression" label instead of right-aligned or in window chrome — it reads as part of the form rather than as a window control.
- **Approach:** Right-align Close in its row or use the window's own close glyph (`open(&mut …)`, as the view editor and notices window do), keeping the `labelled_by` wiring.
- **Acceptance:** The Close affordance sits in a consistent window-chrome position (kittest pin); the field label association stays.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — the close button right-aligns in its row (window-chrome position), the field label association stays, and the rect rides the `expression.close` probe (pinned in the right half of the window)

---

### [x] L61 — The notices dialog separators every entry and encodes severity by color only — 🟢
- **Location:** `crates/datacraft-ui/src/notices_dialog.rs` (the entry loop: `ui.separator()` per entry; severity carried by danger/accent text color).
- **Problem:** Found in the L47 design review (2026-11-19, finding 24): a separator after every entry adds visual noise, and severity is encoded only by text color — no icon or tag, so the encoding fails without color.
- **Approach:** Drop per-entry separators in favour of spacing (or separators only between severity groups), and add a painted severity glyph/tag beside each entry — painted, not a font glyph (T11 policy).
- **Acceptance:** The dialog reads as a list, not a rule table (kittest/layout_probe pin); severity is recognisable without color.
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — the per-entry separators are gone (spacing only) and every entry carries a painted severity glyph beside the text (filled triangle for errors, dot for notices — never a font glyph, the T11 policy); the glyphs ride the `notices.glyph.{error,notice}` probe, pinned per severity count

---

### [x] L62 — Macro-viewer action numbers do not align from ten on — 🟢
- **Location:** `crates/datacraft-ui/src/macro_viewer.rs` (the action list: `ui.weak(format!("{}.", index + 1))`).
- **Problem:** Found in the L47 design review (2026-11-19, finding 25): unpadded numbers (`1.` vs `10.`) shift the monospace argument column — the list jagges.
- **Approach:** Right-align the number column to the widest index (fixed-width formatting matching the digit count).
- **Acceptance:** The argument column keeps one left edge for lists of ten or more entries (kittest pin with a synthetic long macro).
- **Steps:** none yet (from the phase 89 design review).
- **Done:** done in `ed6889a` — the number column width-pads to the widest index and renders monospace (exact width, not proportional spaces); the argument rects ride the `macro.argument` probe, pinned one left edge over 12 fixture actions

## ✅ Done Index

> One-liner per completed item (details above).

- M109 — No version display in any client (UI, CLI, MCP) — done (done in `141bd90` — one shared source `datacraft_core::VERSION` (workspace lockstep): CLI `--version`/`-V` exits 0 printing `datacraft-cli <version>` on stdout before any database work (position-independent); the wire gains the additive `version` method (name + version, no existing contract moves); the backstage Info page shows the row above the database facts, with and without an open database; contract pins in transport/cli_contract/ui_kittest_version over the shared const; de catalog entry added)
- M110 — SQL window grows animated toward full screen height on every run — done (done in `4b532b1` — the shared grid's `max_height` policy: None keeps the datasheet/tab-hosted grids filling (pins unchanged), Some(bound) shrinks to content and caps — the SQL window passes a ten-row bound so the window's auto-size loop is no longer fed; the window records its rect under the `sql.window` layout probe; frame-stability discriminator proves the fix (policy off: window grows 539→709 px between frames 5–15, both tests red))
- M111 — Import File accepts database files and answers with a raw decoder error — done (done in `f1e5e45` — read_csv_file sniffs the head before the decode chain (core `port::sniff_database_family`, the registry's signatures, no new dependency): Jet/ACE and SQLite files answer "this is a <family> database file, not delimited text — open it via File → Open"; the open-file dialog request carries the surface's extensions (import csv/txt/tsv, open mdb/accdb/sqlite/sqlite3/db) through the localized "Supported files" filter; wire pins over both families, kittest pin in the window flow)
- M112 — Import errors repeat the file path twice — done (done in `f1e5e45` — one layer owns the path prefix: read_csv_file keeps its single `{path}: ` wrapper (both the read and the decode branch), and the import_file/preview_import_file handlers forward the reason untouched; the composed decode error carries the path exactly once, pinned with the precise inner location (import_surface.rs))
- L53 — Import source-file field clips long paths at the start — done (done in `f1e5e45` — one path-field contract: labels::path_field_identity renders the chosen file name as its own strong row with the full path on hover, beside the import source field and every backstage path field (open, new-database location, save-a-copy, sidecar); fields widened (import 280→360 at the 520 default window, backstage 360→480); kittest pins all five surfaces at the default size)
- L52 — Status-line errors flood the bar without truncation — done (done in `64da362` — one shared truncation helper (shell::notice_label, crate-visible): binary-searched ellipsis clip over the label font's width math, full text above the click hint on hover, click-to-history unchanged; the status bar and the backstage status line share it; label rect recorded under the `status.notice` layout probe, pinned at both pinned window sizes)
- M113 — Sort order scrambles with NULL values and the indicator can claim unapplied sorts — done (done in `7e35031` — the total comparator `datacraft_types::sort_compare` (NULLs first ascending/last descending via the direction reverse, declaration-order family rank under mixes, non-finite floats tiered after their family; `compare` stays family-safe Option for its callers), `view_order` sorts with it; the stale-sort fallback is gone — a view_order failure clears the sort state through `SetTableSort`-None (whose clear accepts any field name), so the indicator never claims an unapplied order; semantics documented in FEATURES and a COMPATIBILITY matrix row; core fix-forward `86c6452` corrected the property's direction-aware rank assertion)
- M114 — The AutoNumber cell's contract is invisible until a rejection — done (done in `df7de1e` — the gate variant: engine-assigned new-record cells render disabled with the rejection reason on hover (probe `datasheet.newrow.autonumber_gate`), the row hint names the contract precisely in both catalogs, and the row_edit rejection names the column; the M8 writer behaviour untouched; the trailing row sits below the a11y fold headless, so the string pin rides a crate unit test and the gate rides the probe)
- L54 — "Felder fixieren" has no explanation and no visible frozen state — done (done in `df7de1e` — both freeze menu items carry one-line tooltips ("Keep the leading columns pinned…", "Release the pinned columns."), and the frozen state is visible without the menu: a 2 px accent boundary after the last frozen column in header and body, recorded under the `datasheet.frozen_boundary` layout probe and pinned present frozen / absent unfrozen; M39 semantics unchanged)
- M115 — Nav-pane sort menu shows no active order and reads per-group while being global — done (done in `fa378a5` — the sort menu renders the active order as selectable state (the M79 pattern: name/type entries carry selected/unselected marks, pinned via the accesskit toggled property) and the entries name the pane-wide scope ("Sort all sections by Name/Type"); sorting behaviour unchanged; de catalog follows)
- L55 — Nav-pane entry click target excludes the icon — done (done in `fa378a5` — a nav row is one interacted rect (the backstage nav-row pattern): icon and label paint inside it, the row carries the selected widget state, and a click over the icon area opens the object (pinned by a geometric icon-position click); context menu, selection state and automation ids unchanged)
- L56 — Native-objects inventory renders as disabled buttons — done (done in `fa378a5` — the native inventory renders in the pane's row language (muted label row with its kind icon, never disabled button chrome); the row is a real Label widget so the name survives the semantic tree's static-text culling (the M60/M61 note — pinned with a tall-viewport harness); the read-only state carries the group hint and no affordance appears)
- L57 — The nav-pane category dropdown opens a menu that does nothing — done (done in `fa378a5` — the category header is a plain strong labelled header until the category model grows (L48); no menu opens that cannot do anything (LESSONS #29 family); the Access category language returns with L48)
- L58 — The nav pane's "New View" button sits detached at the pane bottom — done (done in `fa378a5` — view creation lives in the Views group header's context menu (below the sort entries); the detached pane-bottom button is gone, and the menu entry opens the view editor (pinned via the open draft))
- M116 — The Save page mixes titled and untitled sections — done (done in `ba97071` — the sidecar section carries the "Design Sidecar" heading (de: Design-Seitendatei) like the other two Save-page sections; no flow change; pinned by the backstage kittest (all three headings present))
- L59 — The Info page's fact values do not align into a column — done (done in `ba97071` — the Info page's fact rows align into a property grid: the widest label measures the shared column edge (strong-render slack absorbed), every value starts there; the value rects ride the `backstage.info.fact_value` layout probe, pinned equal-left at render time; driver facts, the integrity verdict and the lock-file row share the helper)
- M117 — The Relationships window has no scroll area — done (done in `ed6889a` — the mapper's body scrolls within a 380 px height budget; the viewport rect and every row rect ride the layout probe (`relationships.viewport` ≤ budget, all fixture rows laid out inside the scrollable body); canvas geometry math unchanged)
- M118 — VBA viewer paints error text in the literal LIGHT_RED instead of tokens.danger — done (done in `ed6889a` — both error sites paint `tokens.danger` instead of the LIGHT_RED literal; pinned as a source contract over the viewer's source (no color literals); rendering otherwise unchanged)
- M119 — The view editor's fields have no label association — done (done in `ed6889a` — both view-editor fields carry their labels via labelled_by; the accessibility names resolve in the kittest tree (no behaviour change))
- L60 — The expression window's Close button sits mid-title-row — done (done in `ed6889a` — the close button right-aligns in its row (window-chrome position), the field label association stays, and the rect rides the `expression.close` probe (pinned in the right half of the window))
- L61 — The notices dialog separators every entry and encodes severity by color only — done (done in `ed6889a` — the per-entry separators are gone (spacing only) and every entry carries a painted severity glyph beside the text (filled triangle for errors, dot for notices — never a font glyph, the T11 policy); the glyphs ride the `notices.glyph.{error,notice}` probe, pinned per severity count)
- L62 — Macro-viewer action numbers do not align from ten on — done (done in `ed6889a` — the number column width-pads to the widest index and renders monospace (exact width, not proportional spaces); the argument rects ride the `macro.argument` probe, pinned one left edge over 12 fixture actions)
