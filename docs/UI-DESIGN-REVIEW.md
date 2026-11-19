# UI design review — density, typography, spacing (L47)

> Protocol for the human's live review of the running build (phase 89,
> step 89.4, item L47). The review itself changes no code: every accepted
> finding becomes its own `BACKLOG.md` item with location and acceptance,
> and the fix lands in its own released step. Screenshots are optional but
> helpful; "before" captures stay in this log.
>
> Reference discipline: Microsoft's Access support documentation
> (`support.microsoft.com/access`) is the structural idea source — nothing
> is copied, the visual language stays the original token system (ADR-6).
> The token system itself is kept (ARCHITECTURE §12); this pass tunes
> density/typography/spacing within it. No framework or shell rewrite.

## Review conventions

- **Build under review:** current `main` at review start
  (`127bb88` + the phase 89 decision records), release build,
  default window size (1280×800) and the pinned minimum (960×600)
  both checked where layout is affected.
- **Sample data:** the shipped `bike_shop` sample (tables, form, report)
  or the reviewer's own database — whichever exercises the surface.
- **The three questions per surface** (Access density conventions):
  1. **Density** — is the information per screen right? Too airy, too
     cramped, wasted rows, misgrouped controls?
  2. **Typography** — do the type rungs (Inter scale) and weights read
     correctly? Hierarchy of labels vs values vs chrome?
  3. **Spacing** — alignment, padding, gaps: does anything sit oddly,
     overlap, or drift from the grid?
- **One finding per row:** surface, location (what exactly), which of the
  three questions, severity (annoyance / inconsistency / broken feel),
  optional screenshot. Vague impressions ("looks off") are refined on the
  spot or dropped — a finding must be actionable into a BACKLOG item.
- **Verdict per finding:** accepted → BACKLOG item with acceptance;
  rejected → stays in this log with the reason (nothing is lost).

## Surface checklist

| # | Surface | Reviewed | Findings |
|---|---------|----------|----------|
| 1 | Ribbon + Quick Access Toolbar | ✅ 2026-11-17 | none — reviewer: nothing found on the current build |
| 2 | Navigation pane | ✅ 2026-11-19 | findings 11 (M115), 12 (L55), 13 (L56), 14 (L57), 15 (L58); the structural gaps stay with L48 |
| 3 | Datasheet (table view) | ✅ 2026-11-17 | findings 7 (M113), 8 (M114), 9 (L54), 10 (rejected — hard clip is Access parity); further density/typography checkpoints: no findings reported |
| 4 | Backstage (File tab) | ✅ 2026-11-19 | findings 16 (M116), 17 (L59); the version gap is M109; folds: status line → L52, path fields → L53 |
| 5 | Tool windows (SQL, Expression, Relationships, Import, Notices, VBA viewer) | ✅ 2026-11-19 | findings 20 (M117), 21 (L60), 22 (M118), 23 (M119), 24 (L61), 25 (L62); findings 2–6 were logged earlier from out-of-sequence sessions |
| 6 | Form designer + form runtime | ✅ 2026-11-19 | findings 26 (M120), 27 (M121), 28 (M122), 31 (M124 — the reviewer accepted the out-of-class navigation gap: „das brauchen wir") |
| 7 | Report designer + print preview | ✅ 2026-11-19 | findings 29 (M123), 30 (L63) |

All seven surfaces are reviewed (review closed 2026-11-19). Every accepted
finding carries its BACKLOG item; the fixes are fine-packaged as phase 91's
steps 91.7–91.12 in `PROGRESS.md`.

## Findings log

> Appended in review order; one table row per finding. BACKLOG item ids are
> assigned when a finding is accepted.

| # | Surface / location | Question (density/typography/spacing) | Finding | Severity | Verdict | Item |
|---|--------------------|----------------------------------------|---------|----------|---------|------|
| 1 | product-wide — UI (backstage Info), CLI, MCP/JSON-RPC | none of the three — missing affordance | No version display anywhere: the CLI has no `--version` (archived L34 already noted the flag "does not exist in the CLI vocabulary"), the command vocabulary has no About/Version command, and the backstage Info page shows no version. A user or agent cannot tell which build they are talking to. | inconsistency (agent-facing gap) | accepted → item | M109 |
| 2 | tool windows — SQL window (reviewed out of sequence, reviewer's screenshot) | spacing / layout bug | Running any statement grows the window animated toward full screen height — "as if the query were still producing rows" (screenshot: 4 result rows, window fills the screen). Root cause located: the shared grid's `ScrollArea::both().auto_shrink([false, false])` (`datasheet/grid.rs`, correct inside the datasheet CentralPanel) is embedded by `sql_window.rs` without a height budget, so window auto-size and the all-available-space claim feed each other while `show_rows` supplies the virtual row-count height; egui's window-size interpolation makes the growth visible. | broken feel | accepted → item | M110 |
| 3 | tool windows — Import File (reviewer's screenshot) | none of the three — error honesty | A Jet4 `.mdb` chosen in the Import File window (the CSV/text import surface) runs the text-decoder chain (UTF-8 → UTF-16 → Windows-1252) and answers "byte 0x00 at offset 0 is not defined in Windows-1252" — precise but misleading: a healthy database on the wrong surface, with nothing naming the right surface, while the composite's format sniff already knows the file is Jet4. | broken feel (misleading failure) | accepted → item | M111 |
| 4 | tool windows — Import File, error composition (same screenshot) | typography / density of the status line | The composed error shows the file path twice (`C:\…\database-access-ready.mdb: C:\…\database-access-ready.mdb: byte 0x00 …`): `import.rs` wraps the decode error with `"{path}: {e}"` while the inner layer already carries the path — the duplication pushes the actual reason off-screen. | inconsistency | accepted → item | M112 |
| 5 | status bar (same screenshot) | density / typography | A long error paints the full status bar without truncation; the click-into-history affordance exists, but the inline text has no ellipsis/hover contract and floods the chrome. | annoyance | accepted → item | L52 |
| 6 | tool windows — Import File, source-file field (same screenshot) | density / spacing | The Quelldatei field is too narrow for realistic paths: only the tail is visible, the directory context at the start is invisible (the window is resizable but the default is too small). | annoyance | accepted → item | L53 |
| 7 | datasheet — column sort (reviewer's screenshot, reviewed out of sequence) | none of the three — correctness ("Sortierreihenfolge überprüfen") | The Birthday header shows an active descending-sort indicator while the rows follow no column order (dates 1937, 1923, 1935, …, 2026; ids 2, 1, 3, 4, 6, 7; one row has an empty Birthday). Root causes located: `compare()` returns `None` for NULL/mixed-family pairs and `view_order` degrades `None` → `Ordering::Equal`, making the comparator inconsistent (violates the sort contract, scrambles the order); separately, `datasheet.rs` swallows `view_order` errors with `.ok()` yet still renders the indicator from the sort state — indicator-without-sort is possible. Access sorts NULLs at a defined place (first ascending, last descending); here NULL placement is undefined. | broken feel (wrong data order presented as sorted) | accepted → item | M113 |
| 8 | datasheet — new-record hint (reviewer comment) | none of the three — affordance honesty | The hint "Leere Zellen sind Null; AutoWert vergibt sich selbst." leaves it unclear whether a manual value may be entered. It may not: the Jet port rejects user-supplied AutoNumber values (M8), and the datasheet has no AutoNumber edit gate — the contract surfaces only as a runtime rejection after typing. | inconsistency | accepted → item | M114 |
| 9 | datasheet — column context menu, "Felder fixieren" (reviewer comment) | none of the three — affordance clarity | It is unclear what "Felder fixieren" does: no tooltip explains that the leading columns stay pinned during horizontal scroll, and the frozen state itself is not visibly marked in the grid. | annoyance | accepted → item | L54 |
| 10 | datasheet — cell text truncation (reviewer verdict on the clarification) | typography | Cell values clip hard at the column edge without a truncation mark (grid paints cell text clipped, e.g. `margaret@example.con`); an ellipsis alternative was offered. | — | **rejected** — reviewer: „das passt so"; hard clipping is the Access datasheet convention (widen the column), the current behaviour is deliberate parity, no item | — |
| 11 | navigation pane — group-header sort menu (`nav_pane.rs`) | none of the three — affordance honesty | The sort menu offers "Sort by Name"/"Sort by Type" as plain buttons with no indicator of the active order, and the menu sits on every group header while it sets one pane-wide `nav_sort` — reads per-group, acts global. | inconsistency | accepted → item | M115 |
| 12 | navigation pane — entry rows (`nav_pane.rs`) | spacing / affordance | Only the label is the click target; the icon is an inert widget and the 6 px item spacing splits icon and label into two visual widgets — Access opens from the whole row. | annoyance | accepted → item | L55 |
| 13 | navigation pane — native-objects group (`nav_pane.rs`) | typography / chrome | The read-only inventory renders every entry as a disabled button while all other entries are selectable labels — the group reads as a block of dead controls. | annoyance | accepted → item | L56 |
| 14 | navigation pane — category dropdown (`nav_pane.rs`) | none of the three — affordance honesty | The "All Access Objects" dropdown opens a menu whose only entry re-states the current selection and performs nothing — a dead-looking affordance (LESSONS #29 family) until L48's categories exist. | annoyance | accepted → item | L57 |
| 15 | navigation pane — "New View" button (`nav_pane.rs`) | spacing / placement | The button pins below the object list at the pane bottom regardless of any Views group state — an odd duplicate of the ribbon's Views entry point that reads as orphaned chrome. | annoyance | accepted → item | L58 |
| 16 | backstage — Save page (`backstage/save.rs`) | typography | Mixed section hierarchy: "Save a Copy" and "Database Password" carry headings while the sidecar section above starts with a bare field label — three sections, two titled. | inconsistency | accepted → item | M116 |
| 17 | backstage — Info fact rows (`backstage/info.rs`) | density | Fact rows are label+value horizontals; label widths differ, so values start at varying x — the page reads ragged where a property-grid alignment would match the Access Info look. | annoyance | accepted → item | L59 |
| 18 | backstage — `status_line` (`backstage/pages.rs`) | density | Long errors/notes wrap the page bottom without a truncation contract — the same unbounded text pattern as finding 5's status bar. | annoyance | **accepted → folded into L52** (one truncation contract, both call sites) | L52 |
| 19 | backstage — fixed-width path fields (Open/New/Save-As/sidecar, each `desired_width(360.0)`) | density | The same fixed-width pattern as finding 6's import field: realistic paths clip their start in every backstage path field. | annoyance | **accepted → folded into L53** (one path-field contract, all fields) | L53 |
| 20 | tool windows — Relationships (`relationships.rs`) | spacing | The window body (list, create form, canvas) has no ScrollArea — beyond one window height the content clips with no way to reach it. | annoyance | accepted → item | M117 |
| 21 | tool windows — Expression window (`expression_window.rs`) | spacing / placement | The Close button renders directly after the field label in the first row — it reads as part of the form, not as window chrome. | annoyance | accepted → item | L60 |
| 22 | tool windows — VBA viewer error text (`vba_viewer.rs`, two sites) | typography | Error text paints the literal `Color32::LIGHT_RED` instead of `tokens.danger` — breaks the token contract (ADR-6) and is harsh/wrong in dark mode. | inconsistency | accepted → item | M118 |
| 23 | tool windows — view editor (`views_window.rs`) | consistency / a11y | "View name" and "SQL" fields have no `labelled_by` association — the established pattern (expression window, backstage open field) names every field. | inconsistency | accepted → item | M119 |
| 24 | tool windows — notices dialog (`notices_dialog.rs`) | density | A separator after every entry plus severity encoded by text color only — visual noise, color-only encoding. | annoyance | accepted → item | L61 |
| 25 | tool windows — macro viewer (`macro_viewer.rs`) | spacing | Unpadded action numbers (`1.` vs `10.`) shift the monospace argument column — the list jagges from ten on. | annoyance | accepted → item | L62 |
| 26 | form designer/runtime — selected-control ring (`form_design.rs`, shared with the report canvas) | none of the three — state visibility | The ring strokes `selection.bg_fill` (the `checked` wash) — dark `0x1F3A36` on the `0x252525` form surface is nearly invisible, so designers lose the selection state in dark mode; light mode shows only a faint ring. | broken feel | accepted → item | M120 |
| 27 | form runtime — combo dropdown affordance (`form_runtime.rs`) | typography | The dropdown affordance is the literal letter "v" as button text — reads as text, not as a caret; the design canvas paints a proper caret polygon for the same control kind. | inconsistency | accepted → item | M121 |
| 28 | form designer/runtime — fixed 640×400 canvas | spacing | No scroll or zoom: at the pinned 960×600 minimum with the nav pane open the canvas right edge clips; controls beyond are unreachable — designer and runtime alike. | annoyance | accepted → item | M122 |
| 29 | report preview — canvas geometry (`report_design.rs`) | none of the three — honesty | The preview hardcodes A4 595×842 pt while the design canvas renders `report.page` — a sidecar with a different `PageSetup` previews on the wrong paper (latent: no UI edits the page setup, but the field is sidecar-deserialized). | inconsistency | accepted → item | M123 |
| 30 | report preview — text scale (`report_design.rs`) | typography | Geometry scales by 0.9 while glyphs paint at 100 % Body — long values can overflow their scaled rects on the preview page. | annoyance | accepted → item | L63 |
| 31 | form runtime — record navigation row (`form_runtime.rs`) | none of the three — feature gap (out of class, reviewer's call) | The row carries only ‹ Prev / Next › — no first/last/new-record, while the datasheet navigation carries the fuller Access set. | feature gap | **accepted** — reviewer comment: „das brauchen wir"; out-of-class but wanted → item | M124 |
