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
| 2 | Navigation pane | ⬜ | — |
| 3 | Datasheet (table view) | 🔄 2026-11-17 | sort finding 7 (M113); remaining: density/typography/spacing pass |
| 4 | Backstage (File tab) | ⬜ | — |
| 5 | Tool windows (SQL, Expression, Relationships, Import, Notices, VBA viewer) | ⬜ | — |
| 6 | Form designer + form runtime | ⬜ | — |
| 7 | Report designer + print preview | ⬜ | — |

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
